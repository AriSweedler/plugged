use std::sync::atomic::{AtomicI32, Ordering};

pub struct WindowSize {
    pub columns: usize,
    pub rows: usize,
}

pub fn is_terminal(fd: i32) -> bool {
    unsafe { libc::isatty(fd) != 0 }
}

pub fn size() -> WindowSize {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    if unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ as _, &mut ws) } != 0 || ws.ws_col == 0 || ws.ws_row == 0 {
        return WindowSize { columns: 80, rows: 24 };
    }
    WindowSize { columns: ws.ws_col as usize, rows: ws.ws_row as usize }
}

// Straight to the descriptor, whole: a frame is one write, never interleaved with a buffer's.
pub fn write(text: &str) {
    let bytes = text.as_bytes();
    let mut offset = 0;
    while offset < bytes.len() {
        let written = unsafe { libc::write(libc::STDOUT_FILENO, bytes[offset..].as_ptr().cast(), bytes.len() - offset) };
        if written < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return;
        }
        offset += written as usize;
    }
}

pub fn enter_screen() {
    write("\x1b[?1049h\x1b[?25l");
}

pub fn leave_screen() {
    write("\x1b[0m\x1b[?25h\x1b[?1049l");
}

// Home, then each line cleared to its end, then clear below: nothing flashes and nothing scrolls.
pub fn draw(lines: &[String]) {
    let body: Vec<String> = lines.iter().map(|line| format!("{line}\x1b[K")).collect();
    write(&format!("\x1b[H{}\x1b[J", body.join("\r\n")));
}

/// Line buffering and echo go; ISIG stays so Ctrl-C still raises SIGINT, OPOST stays so "\n" still works.
pub struct RawMode {
    original: libc::termios,
}

impl RawMode {
    pub fn enter() -> Option<RawMode> {
        let mut t: libc::termios = unsafe { std::mem::zeroed() };
        if !is_terminal(libc::STDIN_FILENO) || unsafe { libc::tcgetattr(libc::STDIN_FILENO, &mut t) } != 0 {
            return None;
        }
        let original = t;
        t.c_lflag &= !(libc::ICANON | libc::ECHO);
        t.c_cc[libc::VMIN] = 1;
        t.c_cc[libc::VTIME] = 0;
        unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &t) };
        Some(RawMode { original })
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.original) };
    }
}

static SIGNAL_WRITER: AtomicI32 = AtomicI32::new(-1);

extern "C" fn forward_signal(sig: libc::c_int) {
    let byte = sig as u8;
    unsafe { libc::write(SIGNAL_WRITER.load(Ordering::Relaxed), (&byte as *const u8).cast(), 1) };
}

/// The handler only writes the signal number to a pipe, so poll() sees signals the same way it
/// sees keys. Returns the pipe's read end, or -1.
pub fn install_signal_pipe(signals: &[libc::c_int]) -> i32 {
    let mut fds = [-1i32; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return -1;
    }
    SIGNAL_WRITER.store(fds[1], Ordering::Relaxed);
    for &sig in signals {
        unsafe { libc::signal(sig, forward_signal as extern "C" fn(libc::c_int) as libc::sighandler_t) };
    }
    fds[0]
}
