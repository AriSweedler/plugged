use super::terminal::{self, RawMode};
use crate::collect::collect_snapshot;
use crate::model::Snapshot;
use crate::render::{build_frame, compact, render_ansi, render_text, truncate};
use std::io::Read;
use std::time::{Duration, Instant};

pub struct Options {
    pub interval: f64,
    pub from_stdin: bool,
}

/// None means the arguments were not understood.
pub fn parse_options(arguments: &[String]) -> Option<Options> {
    let mut options = Options { interval: 2.0, from_stdin: false };
    let seconds = |value: &str| value.parse::<f64>().ok().filter(|seconds| *seconds > 0.0 && seconds.is_finite());
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "-" => options.from_stdin = true,
            "--interval" => options.interval = seconds(rest.next()?)?,
            other => options.interval = seconds(other.strip_prefix("--interval=")?)?,
        }
    }
    Some(options)
}

/// Bare `plugged`: the dashboard on a terminal. Anywhere else (a pipe, a script, a capture) the
/// text view once, and stdin is never read: only `tui` takes a snapshot from it.
pub fn run_dashboard(options: &Options) -> i32 {
    if !terminal::is_terminal(libc::STDIN_FILENO) || !terminal::is_terminal(libc::STDOUT_FILENO) {
        println!("{}", render_text(&collect_snapshot()));
        return 0;
    }
    run_live(options.interval)
}

pub fn run_tui(options: &Options) -> i32 {
    if options.from_stdin || !terminal::is_terminal(libc::STDIN_FILENO) {
        return render_stdin_once();
    }
    if !terminal::is_terminal(libc::STDOUT_FILENO) {
        println!("{}", render_text(&collect_snapshot()));
        return 0;
    }
    run_live(options.interval)
}

fn render_stdin_once() -> i32 {
    let mut input = Vec::new();
    let read = std::io::stdin().read_to_end(&mut input).map_err(|error| error.to_string());
    let snapshot: Snapshot = match read.and_then(|_| serde_json::from_slice(&input).map_err(|error| error.to_string())) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("plugged tui: stdin is not a plugged JSON document ({error})");
            return 1;
        }
    };
    if !terminal::is_terminal(libc::STDOUT_FILENO) {
        println!("{}", render_text(&snapshot));
        return 0;
    }
    let columns = terminal::size().columns;
    let lines: Vec<String> = build_frame(&snapshot, "stdin").iter().map(|line| render_ansi(&truncate(line, columns))).collect();
    println!("{}", lines.join("\n"));
    0
}

struct Screen;

impl Drop for Screen {
    fn drop(&mut self) {
        terminal::leave_screen();
    }
}

fn run_live(interval: f64) -> i32 {
    let signal_fd = terminal::install_signal_pipe(&[libc::SIGINT, libc::SIGTERM, libc::SIGHUP, libc::SIGWINCH]);
    // Dropped in reverse: the screen is left, then the terminal's modes come back.
    let _raw = RawMode::enter();
    terminal::enter_screen();
    let _screen = Screen;

    let mut size = terminal::size();
    let mut snapshot = collect_snapshot();
    let status = format!("every {} s  q quits", compact(interval));
    let draw = |snapshot: &Snapshot, size: &terminal::WindowSize| {
        let lines: Vec<String> = build_frame(snapshot, &status).iter().take(size.rows).map(|line| render_ansi(&truncate(line, size.columns))).collect();
        terminal::draw(&lines);
    };
    draw(&snapshot, &size);

    let period = Duration::from_secs_f64(interval);
    let mut next_tick = Instant::now() + period;
    let mut fds = [
        libc::pollfd { fd: libc::STDIN_FILENO, events: libc::POLLIN, revents: 0 },
        libc::pollfd { fd: signal_fd, events: libc::POLLIN, revents: 0 },
    ];
    let mut buffer = [0u8; 64];
    loop {
        let wait = next_tick.saturating_duration_since(Instant::now());
        let millis = (wait.as_secs_f64() * 1000.0).ceil().min(i32::MAX as f64) as i32;
        let ready = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, millis) };
        if ready < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return 1;
        }
        if ready == 0 || Instant::now() >= next_tick {
            snapshot = collect_snapshot();
            draw(&snapshot, &size);
            next_tick = Instant::now() + period;
        }
        if fds[0].revents & libc::POLLIN != 0 {
            let count = unsafe { libc::read(libc::STDIN_FILENO, buffer.as_mut_ptr().cast(), buffer.len()) };
            // q, Q, or a raw Ctrl-C byte in case ISIG was already off.
            if count > 0 && buffer[..count as usize].iter().any(|&byte| byte == b'q' || byte == b'Q' || byte == 0x03) {
                return 0;
            }
        }
        if fds[1].revents & libc::POLLIN != 0 {
            let count = unsafe { libc::read(signal_fd, buffer.as_mut_ptr().cast(), buffer.len()) };
            for &sig in &buffer[..count.max(0) as usize] {
                if sig as libc::c_int == libc::SIGWINCH {
                    size = terminal::size();
                    draw(&snapshot, &size);
                } else {
                    return 0;
                }
            }
        }
    }
}
