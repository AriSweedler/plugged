import Foundation

struct WindowSize {
    let columns: Int
    let rows: Int
}

enum Terminal {
    static func size() -> WindowSize {
        var ws = winsize()
        guard ioctl(STDOUT_FILENO, TIOCGWINSZ, &ws) == 0, ws.ws_col > 0, ws.ws_row > 0 else {
            return WindowSize(columns: 80, rows: 24)
        }
        return WindowSize(columns: Int(ws.ws_col), rows: Int(ws.ws_row))
    }

    static func write(_ text: String) {
        Array(text.utf8).withUnsafeBytes { buffer in
            guard let base = buffer.baseAddress else { return }
            var offset = 0
            while offset < buffer.count {
                let written = Darwin.write(STDOUT_FILENO, base + offset, buffer.count - offset)
                if written < 0 {
                    if errno == EINTR { continue }
                    return
                }
                offset += written
            }
        }
    }
}

enum Screen {
    static func enter() {
        Terminal.write("\u{1B}[?1049h\u{1B}[?25l")
    }

    static func leave() {
        Terminal.write("\u{1B}[0m\u{1B}[?25h\u{1B}[?1049l")
    }

    // Home, then each line cleared to its end, then clear below: nothing flashes and nothing scrolls.
    static func draw(_ lines: [String]) {
        Terminal.write("\u{1B}[H" + lines.map { $0 + "\u{1B}[K" }.joined(separator: "\r\n") + "\u{1B}[J")
    }
}

// Line buffering and echo go; ISIG stays so Ctrl-C still raises SIGINT, OPOST stays so "\n" still works.
struct RawMode {
    private let original: termios

    init?() {
        var t = termios()
        guard isatty(STDIN_FILENO) != 0, tcgetattr(STDIN_FILENO, &t) == 0 else { return nil }
        original = t
        t.c_lflag &= ~tcflag_t(ICANON | ECHO)
        withUnsafeMutablePointer(to: &t.c_cc) { cc in
            cc.withMemoryRebound(to: cc_t.self, capacity: Int(NCCS)) { cc in
                cc[Int(VMIN)] = 1
                cc[Int(VTIME)] = 0
            }
        }
        tcsetattr(STDIN_FILENO, TCSANOW, &t)
    }

    func restore() {
        var t = original
        tcsetattr(STDIN_FILENO, TCSANOW, &t)
    }
}

nonisolated(unsafe) private var signalWriter: Int32 = -1

private func forwardSignal(_ sig: Int32) {
    var byte = UInt8(truncatingIfNeeded: sig)
    _ = write(signalWriter, &byte, 1)
}

// The handler only writes the signal number to a pipe, so poll() sees signals the same way it sees keys.
// Returns the pipe's read end, or -1.
func installSignalPipe(_ signals: [Int32]) -> Int32 {
    var fds: [Int32] = [-1, -1]
    guard pipe(&fds) == 0 else { return -1 }
    signalWriter = fds[1]
    for sig in signals { signal(sig, forwardSignal) }
    return fds[0]
}
