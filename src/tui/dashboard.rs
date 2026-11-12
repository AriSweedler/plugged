use super::terminal::{self, RawMode};
use crate::collect::collect_snapshot;
use crate::model::Snapshot;
use crate::render::{build_frame, compact, render_ansi, render_plain, render_text};
use std::io::Read;
use std::time::{Duration, Instant};

pub struct Options {
    pub interval: f64,
    pub from_stdin: bool,
    /// Draw the frame this wide, whatever the terminal says; off a terminal, draw it plain.
    pub width: Option<usize>,
}

/// None means the arguments were not understood.
pub fn parse_options(arguments: &[String]) -> Option<Options> {
    let mut options = Options { interval: 2.0, from_stdin: false, width: None };
    let seconds = |value: &str| value.parse::<f64>().ok().filter(|seconds| *seconds > 0.0 && seconds.is_finite());
    let mut rest = arguments.iter();
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "-" => options.from_stdin = true,
            "--interval" => options.interval = seconds(rest.next()?)?,
            "--width" => options.width = Some(rest.next()?.parse::<usize>().ok().filter(|width| *width > 0)?),
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
        return match read_stdin() {
            Ok(snapshot) => render_once(&snapshot, "stdin", options),
            Err(error) => {
                eprintln!("plugged tui: stdin is not a plugged JSON document ({error})");
                1
            }
        };
    }
    if !terminal::is_terminal(libc::STDOUT_FILENO) {
        return render_once(&collect_snapshot(), "once", options);
    }
    run_live(options.interval)
}

fn read_stdin() -> Result<Snapshot, String> {
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input).map_err(|error| error.to_string())?;
    serde_json::from_slice(&input).map_err(|error| error.to_string())
}

// One frame as ordinary lines, never the alternate screen: colored on a terminal; off one, the
// text view, or with --width the frame without its colors (what tests/ compares).
fn render_once(snapshot: &Snapshot, status: &str, options: &Options) -> i32 {
    let on_terminal = terminal::is_terminal(libc::STDOUT_FILENO);
    let lines: Vec<String> = match (on_terminal, options.width) {
        (true, width) => build_frame(snapshot, status, width.unwrap_or_else(|| terminal::size().columns)).iter().map(render_ansi).collect(),
        (false, Some(width)) => build_frame(snapshot, status, width).iter().map(render_plain).collect(),
        (false, None) => vec![render_text(snapshot)],
    };
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
        let lines: Vec<String> = build_frame(snapshot, &status, size.columns).iter().take(size.rows).map(render_ansi).collect();
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
