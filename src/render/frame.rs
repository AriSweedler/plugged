use super::merge::merge_containers;
use super::{compact, device_names, power_state, refresh, width};
use crate::model::{Display, Port, Power, Snapshot, UsbDevice};

#[derive(Clone, Copy)]
pub enum Color {
    Red = 1,
    Green = 2,
    Yellow = 3,
}

#[derive(Clone, Copy, Default)]
pub struct Style {
    color: Option<Color>,
    bold: bool,
    dim: bool,
}

impl Style {
    const PLAIN: Style = Style { color: None, bold: false, dim: false };
    const BOLD: Style = Style { color: None, bold: true, dim: false };
    const DIM: Style = Style { color: None, bold: false, dim: true };
    const fn fg(color: Color) -> Style {
        Style { color: Some(color), bold: false, dim: false }
    }

    fn sgr(self) -> String {
        let mut codes: Vec<String> = Vec::new();
        if self.bold {
            codes.push("1".to_string());
        }
        if self.dim {
            codes.push("2".to_string());
        }
        if let Some(color) = self.color {
            codes.push((30 + color as u8).to_string());
        }
        if codes.is_empty() { String::new() } else { format!("\x1b[{}m", codes.join(";")) }
    }
}

#[derive(Clone)]
pub struct Span {
    text: String,
    style: Style,
}

fn span(text: impl Into<String>, style: Style) -> Span {
    Span { text: text.into(), style }
}

fn plain(text: impl Into<String>) -> Span {
    span(text, Style::PLAIN)
}

pub type Line = Vec<Span>;

pub fn build_frame(s: &Snapshot, status: &str) -> Vec<Line> {
    let devices = merge_containers(&s.usb);
    let none = || vec![vec![span("  (none)", Style::DIM)]];
    let mut lines: Vec<Line> = vec![
        vec![span("plugged", Style::BOLD), plain(format!("  {}  ", s.host)), span(format!("{}  {}", clock_time(&s.generated_at), status), Style::DIM)],
        vec![],
        vec![span("Ports", Style::BOLD)],
    ];
    lines.extend(if s.ports.is_empty() { none() } else { table(s.ports.iter().map(|port| port_cells(port, &devices)).collect()) });
    lines.extend([vec![], vec![span("Devices", Style::BOLD)]]);
    lines.extend(if devices.is_empty() { none() } else { table(devices.iter().flat_map(|device| device_cells(device, &s.ports, 1)).collect()) });
    lines.extend([vec![], displays_line(&s.displays), power_line(&s.power)]);
    lines
}

/// Every glyph is one cell wide by construction, so a character count is a column count.
pub fn truncate(line: &Line, to: usize) -> Line {
    let mut remaining = to;
    let mut out: Line = Vec::new();
    for piece in line {
        if remaining == 0 {
            break;
        }
        let count = width(&piece.text);
        if count <= remaining {
            out.push(piece.clone());
            remaining -= count;
        } else {
            out.push(span(piece.text.chars().take(remaining).collect::<String>(), piece.style));
            remaining = 0;
        }
    }
    out
}

pub fn render_ansi(line: &Line) -> String {
    line.iter()
        .map(|piece| {
            let sgr = piece.style.sgr();
            if sgr.is_empty() { piece.text.clone() } else { format!("{sgr}{}\x1b[0m", piece.text) }
        })
        .collect()
}

fn clock_time(iso8601: &str) -> String {
    iso8601.chars().skip(11).take(8).collect()
}

fn port_cells(port: &Port, devices: &[UsbDevice]) -> Line {
    vec![
        plain(format!("  {}", port.label)),
        if port.active { span("\u{25CF}", Style::fg(Color::Green)) } else { span("\u{25CB}", Style::DIM) },
        plain(port.transports.join(" ")),
        plain(device_names(port, devices).join(", ")),
        span(if port.power_in == Some(true) { "power-in" } else { "" }, Style::fg(Color::Yellow)),
    ]
}

fn device_cells(device: &UsbDevice, ports: &[Port], depth: usize) -> Vec<Line> {
    let port_label = if depth == 1 {
        ports.iter().find(|port| port.id == device.port).map_or(device.port.clone(), |port| port.label.clone())
    } else {
        String::new()
    };
    let mut rows = vec![vec![plain(format!("{}{}", "  ".repeat(depth), device.name)), span(device.speed_label.clone(), Style::DIM), span(port_label, Style::DIM)]];
    rows.extend(device.children.iter().flat_map(|child| device_cells(child, ports, depth + 1)));
    rows
}

fn displays_line(displays: &[Display]) -> Line {
    let mut line: Line = vec![span("Displays  ", Style::BOLD)];
    if displays.is_empty() {
        line.push(span("(none)", Style::DIM));
        return line;
    }
    for (i, d) in displays.iter().enumerate() {
        if i > 0 {
            line.push(span("  |  ", Style::DIM));
        }
        line.push(plain(format!("{} {}x{}{}", d.name, d.width, d.height, refresh(d))));
        let tags: Vec<&str> = [(d.builtin, "builtin"), (d.main, "main")].iter().filter(|(on, _)| *on).map(|(_, tag)| *tag).collect();
        if !tags.is_empty() {
            line.push(span(format!(" {}", tags.join(" ")), Style::DIM));
        }
    }
    line
}

fn power_line(p: &Power) -> Line {
    let low = p.percent < 20 && !p.external_connected;
    let state_style = if p.fully_charged || p.charging {
        Style::fg(Color::Green)
    } else if p.external_connected {
        Style::PLAIN
    } else {
        Style::fg(Color::Yellow)
    };
    let mut line: Line = vec![
        span("Power     ", Style::BOLD),
        span(format!("battery {}%", p.percent), if low { Style::fg(Color::Red) } else { Style::PLAIN }),
        plain("  "),
        span(power_state(p), state_style),
    ];
    if let Some(a) = &p.adapter {
        line.push(span("  |  ", Style::DIM));
        line.push(plain(format!("adapter {} W ({} V x {} A) {}", a.watts, compact(a.volts), compact(a.amps), a.description)));
    }
    line
}

fn table(rows: Vec<Line>) -> Vec<Line> {
    let columns = rows.first().map_or(0, Vec::len);
    let widths: Vec<usize> = (0..columns).map(|c| rows.iter().map(|row| width(&row[c].text)).max().unwrap_or(0)).collect();
    rows.iter()
        .map(|row| {
            let mut line: Line = Vec::new();
            for (c, cell) in row.iter().enumerate() {
                if c > 0 {
                    line.push(plain("  "));
                }
                line.push(if c == columns - 1 { cell.clone() } else { span(format!("{:<w$}", cell.text, w = widths[c]), cell.style) });
            }
            line
        })
        .collect()
}
