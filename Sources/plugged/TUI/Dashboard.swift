import Foundation

struct TUIOptions {
    var interval: Double = 2
    var fromStdin = false
}

// nil means the arguments were not understood.
func parseTUIOptions(_ arguments: ArraySlice<String>) -> TUIOptions? {
    var options = TUIOptions()
    var rest = arguments.makeIterator()
    while let argument = rest.next() {
        switch argument {
        case "-":
            options.fromStdin = true
        case "--interval":
            guard let value = rest.next(), let seconds = Double(value), seconds > 0 else { return nil }
            options.interval = seconds
        default:
            guard argument.hasPrefix("--interval="),
                  let seconds = Double(argument.dropFirst("--interval=".count)), seconds > 0 else { return nil }
            options.interval = seconds
        }
    }
    return options
}

@MainActor
func runTUI(_ options: TUIOptions) -> Int32 {
    if options.fromStdin || isatty(STDIN_FILENO) == 0 {
        return renderStdinOnce()
    }
    guard isatty(STDOUT_FILENO) != 0 else {
        print(renderText(collectSnapshot()))
        return 0
    }
    return runLive(interval: options.interval)
}

private func renderStdinOnce() -> Int32 {
    let snapshot: Snapshot
    do {
        snapshot = try JSONDecoder().decode(Snapshot.self, from: FileHandle.standardInput.readDataToEndOfFile())
    } catch {
        FileHandle.standardError.write(Data("plugged tui: stdin is not a plugged JSON document (\(error.localizedDescription))\n".utf8))
        return 1
    }
    guard isatty(STDOUT_FILENO) != 0 else {
        print(renderText(snapshot))
        return 0
    }
    let width = Terminal.size().columns
    print(buildFrame(snapshot, status: "stdin").map { renderANSI(truncate($0, to: width)) }.joined(separator: "\n"))
    return 0
}

@MainActor
private func runLive(interval: Double) -> Int32 {
    let signalFD = installSignalPipe([SIGINT, SIGTERM, SIGHUP, SIGWINCH])
    let raw = RawMode()
    Screen.enter()
    defer {
        Screen.leave()
        raw?.restore()
    }

    var size = Terminal.size()
    var snapshot = collectSnapshot()
    let status = "every \(compact(interval)) s  q quits"
    func draw() {
        Screen.draw(buildFrame(snapshot, status: status).prefix(size.rows).map { renderANSI(truncate($0, to: size.columns)) })
    }
    draw()

    var nextTick = Date().addingTimeInterval(interval)
    var fds = [
        pollfd(fd: STDIN_FILENO, events: Int16(POLLIN), revents: 0),
        pollfd(fd: signalFD, events: Int16(POLLIN), revents: 0),
    ]
    var buffer = [UInt8](repeating: 0, count: 64)
    while true {
        let wait = max(0, nextTick.timeIntervalSinceNow)
        let ready = poll(&fds, nfds_t(fds.count), Int32(min((wait * 1000).rounded(.up), Double(Int32.max))))
        if ready < 0 {
            if errno == EINTR { continue }
            return 1
        }
        if ready == 0 || Date() >= nextTick {
            snapshot = collectSnapshot()
            draw()
            nextTick = Date().addingTimeInterval(interval)
        }
        if fds[0].revents & Int16(POLLIN) != 0 {
            let count = read(STDIN_FILENO, &buffer, buffer.count)
            // q, Q, or a raw Ctrl-C byte in case ISIG was already off.
            if count > 0 && buffer[..<count].contains(where: { $0 == 0x71 || $0 == 0x51 || $0 == 0x03 }) { return 0 }
        }
        if fds[1].revents & Int16(POLLIN) != 0 {
            let count = read(signalFD, &buffer, buffer.count)
            for sig in buffer[..<max(count, 0)] {
                if Int32(sig) == SIGWINCH {
                    size = Terminal.size()
                    draw()
                } else {
                    return 0
                }
            }
        }
    }
}
