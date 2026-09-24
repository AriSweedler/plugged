import Foundation

@MainActor
func collectSnapshot() -> Snapshot {
    Snapshot(
        generatedAt: timestamp(),
        host: hostName(),
        ports: collectPorts(),
        usb: collectUSB(),
        displays: collectDisplays(),
        power: collectPower())
}

private func timestamp() -> String {
    let clock = ISO8601DateFormatter()
    clock.timeZone = .current
    clock.formatOptions = [.withInternetDateTime]
    return clock.string(from: Date())
}

// gethostname never touches DNS, unlike ProcessInfo.hostName.
private func hostName() -> String {
    var buffer = [CChar](repeating: 0, count: 256)
    guard gethostname(&buffer, buffer.count) == 0 else { return "" }
    return cString(buffer)
}
