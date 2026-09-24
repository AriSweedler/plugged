import Foundation

func renderText(_ s: Snapshot) -> String {
    let devices = mergeContainers(s.usb)
    var lines = ["Ports"]
    lines += table(s.ports.map { portRow($0, devices) })
    lines += ["", "USB"]
    lines += devices.isEmpty ? ["  (none)"] : table(devices.flatMap { usbRows($0, s.ports, depth: 1) })
    lines += ["", "Displays"]
    lines += s.displays.isEmpty ? ["  (none)"] : s.displays.map(displayLine)
    lines += ["", "Power"]
    lines += powerLines(s.power)
    return lines.joined(separator: "\n")
}

func deviceNames(on port: Port, _ devices: [USBDevice]) -> [String] {
    var names: [String] = []
    for device in devices where device.port == port.id && !names.contains(device.name) {
        names.append(device.name)
    }
    return names
}

func powerState(_ p: Power) -> String {
    var state: String
    if p.fullyCharged {
        state = "full"
    } else if p.charging {
        state = "charging"
    } else if p.externalConnected {
        state = "external power, not charging"
    } else {
        state = "on battery"
    }
    if let minutes = p.timeRemainingMin {
        state += p.charging ? ", \(minutes) min to full" : ", \(minutes) min left"
    }
    return state
}

func compact(_ value: Double) -> String {
    String(format: "%g", value)
}

private func portRow(_ port: Port, _ devices: [USBDevice]) -> [String] {
    [
        "  " + port.label,
        port.active ? "\u{25CF}" : "\u{25CB}",
        port.transports.isEmpty ? "-" : port.transports.joined(separator: " "),
        deviceNames(on: port, devices).joined(separator: ", "),
        port.powerIn == true ? "power-in" : "",
    ]
}

private func usbRows(_ device: USBDevice, _ ports: [Port], depth: Int) -> [[String]] {
    let portLabel = depth == 1 ? (ports.first { $0.id == device.port }?.label ?? device.port) : ""
    let row = [String(repeating: "  ", count: depth) + device.name, device.speedLabel, portLabel]
    return [row] + device.children.flatMap { usbRows($0, ports, depth: depth + 1) }
}

private func displayLine(_ d: Display) -> String {
    var tags: [String] = []
    if d.builtin { tags.append("builtin") }
    if d.main { tags.append("main") }
    return "  \(d.name)  \(d.width)x\(d.height) @ \(compact(d.hz)) Hz" + (tags.isEmpty ? "" : "  " + tags.joined(separator: " "))
}

private func powerLines(_ p: Power) -> [String] {
    var lines = ["  battery \(p.percent)%  \(powerState(p))"]
    if let a = p.adapter {
        lines.append("  adapter \(a.watts) W  \(compact(a.volts)) V x \(compact(a.amps)) A  \(a.description)")
    }
    return lines
}

// Pads every column but the last to the widest cell in it, then trims trailing blanks.
private func table(_ rows: [[String]]) -> [String] {
    guard let columns = rows.first?.count else { return [] }
    let widths = (0..<columns).map { c in rows.map { $0[c].count }.max() ?? 0 }
    return rows.map { row in
        row.enumerated().map { c, cell in
            c == columns - 1 ? cell : cell.padding(toLength: widths[c], withPad: " ", startingAt: 0)
        }
        .joined(separator: "  ")
        .replacingOccurrences(of: "\\s+$", with: "", options: .regularExpression)
    }
}
