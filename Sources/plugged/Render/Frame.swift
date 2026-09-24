import Foundation

enum Color: Int {
    case black = 0, red, green, yellow, blue, magenta, cyan, white
}

struct Style {
    var color: Color?
    var bold = false
    var dim = false

    static let plain = Style()
    static let bold = Style(bold: true)
    static let dim = Style(dim: true)
    static func fg(_ color: Color) -> Style { Style(color: color) }

    var sgr: String {
        var codes: [Int] = []
        if bold { codes.append(1) }
        if dim { codes.append(2) }
        if let color { codes.append(30 + color.rawValue) }
        return codes.isEmpty ? "" : "\u{1B}[" + codes.map(String.init).joined(separator: ";") + "m"
    }
}

struct Span {
    let text: String
    let style: Style

    init(_ text: String, _ style: Style = .plain) {
        self.text = text
        self.style = style
    }
}

typealias Line = [Span]

func buildFrame(_ s: Snapshot, status: String) -> [Line] {
    let devices = mergeContainers(s.usb)
    var lines: [Line] = [
        [Span("plugged", .bold), Span("  \(s.host)  "), Span("\(clockTime(s.generatedAt))  \(status)", .dim)],
        [],
        [Span("Ports", .bold)],
    ]
    lines += table(s.ports.map { portCells($0, devices) })
    lines += [[], [Span("Devices", .bold)]]
    lines += devices.isEmpty ? [[Span("  (none)", .dim)]] : table(devices.flatMap { deviceCells($0, s.ports, depth: 1) })
    lines += [[], displaysLine(s.displays), powerLine(s.power)]
    return lines
}

// Every glyph is one cell wide by construction, so a character count is a column count.
func truncate(_ line: Line, to width: Int) -> Line {
    var remaining = width
    var out: Line = []
    for span in line {
        if remaining <= 0 { break }
        if span.text.count <= remaining {
            out.append(span)
            remaining -= span.text.count
        } else {
            out.append(Span(String(span.text.prefix(remaining)), span.style))
            remaining = 0
        }
    }
    return out
}

func renderANSI(_ line: Line) -> String {
    line.map { span in
        let sgr = span.style.sgr
        return sgr.isEmpty ? span.text : sgr + span.text + "\u{1B}[0m"
    }
    .joined()
}

private func clockTime(_ iso8601: String) -> String {
    String(iso8601.dropFirst(11).prefix(8))
}

private func portCells(_ port: Port, _ devices: [USBDevice]) -> Line {
    [
        Span("  " + port.label),
        port.active ? Span("\u{25CF}", .fg(.green)) : Span("\u{25CB}", .dim),
        Span(port.transports.joined(separator: " ")),
        Span(deviceNames(on: port, devices).joined(separator: ", ")),
        Span(port.powerIn == true ? "power-in" : "", .fg(.yellow)),
    ]
}

private func deviceCells(_ device: USBDevice, _ ports: [Port], depth: Int) -> [Line] {
    let portLabel = depth == 1 ? (ports.first { $0.id == device.port }?.label ?? device.port) : ""
    let row: Line = [
        Span(String(repeating: "  ", count: depth) + device.name),
        Span(device.speedLabel, .dim),
        Span(portLabel, .dim),
    ]
    return [row] + device.children.flatMap { deviceCells($0, ports, depth: depth + 1) }
}

private func displaysLine(_ displays: [Display]) -> Line {
    var line: Line = [Span("Displays  ", .bold)]
    guard !displays.isEmpty else { return line + [Span("(none)", .dim)] }
    for (i, d) in displays.enumerated() {
        if i > 0 { line.append(Span("  |  ", .dim)) }
        line.append(Span("\(d.name) \(d.width)x\(d.height) @ \(compact(d.hz)) Hz"))
        var tags: [String] = []
        if d.builtin { tags.append("builtin") }
        if d.main { tags.append("main") }
        if !tags.isEmpty { line.append(Span(" " + tags.joined(separator: " "), .dim)) }
    }
    return line
}

private func powerLine(_ p: Power) -> Line {
    let low = p.percent < 20 && !p.externalConnected
    let stateStyle: Style = p.fullyCharged || p.charging ? .fg(.green) : p.externalConnected ? .plain : .fg(.yellow)
    var line: Line = [
        Span("Power     ", .bold),
        Span("battery \(p.percent)%", low ? .fg(.red) : .plain),
        Span("  "),
        Span(powerState(p), stateStyle),
    ]
    if let a = p.adapter {
        line.append(Span("  |  ", .dim))
        line.append(Span("adapter \(a.watts) W (\(compact(a.volts)) V x \(compact(a.amps)) A) \(a.description)"))
    }
    return line
}

private func table(_ rows: [Line]) -> [Line] {
    guard let columns = rows.first?.count else { return [] }
    let widths = (0..<columns).map { c in rows.map { $0[c].text.count }.max() ?? 0 }
    return rows.map { row in
        var line: Line = []
        for (c, cell) in row.enumerated() {
            if c > 0 { line.append(Span("  ")) }
            line.append(c == columns - 1 ? cell : Span(cell.text.padding(toLength: widths[c], withPad: " ", startingAt: 0), cell.style))
        }
        return line
    }
}
