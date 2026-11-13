//! The dashboard's frame: one box of stacked panels, built as a grid of styled cells so a link
//! can be drawn across panel borders, then emitted as ANSI (or plain, for tests and pipes).

use super::merge::merge_containers;
use super::{carried_interfaces, carrier_name, compact, detail, device_names, feeding_port, label, link_rate, linked_interface, power_state, refresh};
use crate::model::{Display, Interface, Port, Power, Snapshot, UsbDevice};

#[derive(Clone, Copy, PartialEq, Default)]
pub struct Style {
    color: Option<u8>,
    bold: bool,
    dim: bool,
}

const RED: u8 = 1;
const GREEN: u8 = 2;
const YELLOW: u8 = 3;
const BLUE: u8 = 4;
const MAGENTA: u8 = 5;
const CYAN: u8 = 6;

const PLAIN: Style = Style { color: None, bold: false, dim: false };
const BOLD: Style = Style { color: None, bold: true, dim: false };
const DIM: Style = Style { color: None, bold: false, dim: true };
const fn fg(color: u8) -> Style {
    Style { color: Some(color), bold: false, dim: false }
}
const fn strong(color: u8) -> Style {
    Style { color: Some(color), bold: true, dim: false }
}
// Power is yellow everywhere: the port it comes in on, the link, and the panel it lands in.
const POWER: Style = strong(YELLOW);
// The network is blue the same way: the device it comes in on, the link, and its panel.
const NETWORK: Style = strong(BLUE);

impl Style {
    // The 16 named colors only, so the terminal's own palette decides what they look like.
    fn sgr(self) -> String {
        let mut codes: Vec<String> = Vec::new();
        if self.bold {
            codes.push("1".to_string());
        }
        if self.dim {
            codes.push("2".to_string());
        }
        if let Some(color) = self.color {
            codes.push((30 + color).to_string());
        }
        if codes.is_empty() { String::new() } else { format!("\x1b[{}m", codes.join(";")) }
    }
}

#[derive(Clone, Copy)]
pub struct Cell {
    ch: char,
    style: Style,
}

/// One screen line. Every glyph is one cell wide by construction, so a cell is a column.
pub type Row = Vec<Cell>;

fn cells(text: &str, style: Style) -> Row {
    text.chars().map(|ch| Cell { ch, style }).collect()
}

fn join(parts: Vec<Row>) -> Row {
    parts.into_iter().flatten().collect()
}

// Exactly `to` cells: cut, or padded with blanks.
fn fit(mut row: Row, to: usize) -> Row {
    row.truncate(to);
    row.resize(to, Cell { ch: ' ', style: PLAIN });
    row
}

pub fn truncate(row: &Row, to: usize) -> Row {
    row.iter().take(to).copied().collect()
}

/// Runs of one style share one escape, and each run is reset, so a cut line leaves nothing open.
pub fn render_ansi(row: &Row) -> String {
    let mut out = String::new();
    let mut start = 0;
    while start < row.len() {
        let style = row[start].style;
        let end = (start..row.len()).find(|&i| row[i].style != style).unwrap_or(row.len());
        let text: String = row[start..end].iter().map(|cell| cell.ch).collect();
        let sgr = style.sgr();
        if sgr.is_empty() {
            out.push_str(&text);
        } else {
            out.push_str(&format!("{sgr}{text}\x1b[0m"));
        }
        start = end;
    }
    out
}

pub fn render_plain(row: &Row) -> String {
    row.iter().map(|cell| cell.ch).collect::<String>().trim_end().to_string()
}

struct Panel {
    title: &'static str,
    rows: Vec<Row>,
}

/// A line drawn across the panels: out of one panel's row, down the gutter's rail, and into
/// another's. Both links run down the one rail, and the panel order keeps them apart: the
/// power link spans Ports to Power, above the network link's Devices to Network.
struct Link {
    from: (usize, usize),
    to: (usize, usize),
    style: Style,
}

const WIDEST: usize = 96;
const NARROWEST: usize = 48;

pub fn build_frame(s: &Snapshot, status: &str, columns: usize) -> Vec<Row> {
    let w = columns.clamp(NARROWEST, WIDEST);
    let devices = merge_containers(&s.usb);
    let feeding = feeding_port(&s.ports);

    let mut panels: Vec<Panel> = Vec::new();
    let mut links: Vec<Link> = Vec::new();
    if !s.ports.is_empty() {
        panels.push(Panel { title: "Ports", rows: port_rows(&s.ports, &devices, &s.power) });
    }
    let (power, source_row) = power_rows(&s.power, feeding);
    let power_panel = panels.len();
    panels.push(Panel { title: "Power", rows: power });
    // The power link: out of the row of the port that feeds the machine, into the source line.
    if let (Some(port), Some(source_row)) = (feeding, source_row) {
        if let Some(port_row) = s.ports.iter().position(|candidate| candidate.id == port.id) {
            links.push(Link { from: (0, port_row), to: (power_panel, source_row), style: POWER });
        }
    }
    let (rows, locations) = device_rows(&devices, &s.ports, &s.network);
    let devices_panel = panels.len();
    panels.push(Panel { title: "Devices", rows: if devices.is_empty() { none() } else { rows } });
    // The network link: out of the row of the device the network comes in on, into its interface.
    if !s.network.is_empty() {
        let network_panel = panels.len();
        panels.push(Panel { title: "Network", rows: network_rows(&s.network, &devices) });
        if let Some(interface) = linked_interface(&s.network) {
            let device_row = interface.usb_location_id.as_deref().and_then(|location| locations.iter().position(|carried| carried == location));
            let interface_row = s.network.iter().position(|candidate| candidate.name == interface.name);
            if let (Some(device_row), Some(interface_row)) = (device_row, interface_row) {
                links.push(Link { from: (devices_panel, device_row), to: (network_panel, interface_row), style: NETWORK });
            }
        }
    }
    panels.push(Panel { title: "Displays", rows: if s.displays.is_empty() { none() } else { display_rows(&s.displays) } });

    let mut frame = vec![header(s, status, w), Vec::new()];
    let mut placed: Vec<Vec<usize>> = Vec::new(); // frame line of each panel's rows
    for (index, panel) in panels.iter().enumerate() {
        frame.push(border(if index == 0 { '\u{256D}' } else { '\u{251C}' }, if index == 0 { '\u{256E}' } else { '\u{2524}' }, Some(panel.title), w));
        placed.push((0..panel.rows.len()).map(|row| frame.len() + row).collect());
        frame.extend(panel.rows.iter().map(|row| boxed(row.clone(), w)));
    }
    frame.push(border('\u{2570}', '\u{256F}', None, w));
    for link in &links {
        draw_link(&mut frame, placed[link.from.0][link.from.1], placed[link.to.0][link.to.1], w, link.style);
    }
    if columns < w { frame.iter().map(|row| truncate(row, columns)).collect() } else { frame }
}

fn none() -> Vec<Row> {
    vec![cells("(none)", DIM)]
}

fn header(s: &Snapshot, status: &str, w: usize) -> Row {
    let left = join(vec![cells(" plugged", strong(CYAN)), cells(&format!("  {}", s.host), BOLD)]);
    let right = cells(&format!("{}  \u{00B7}  {} ", clock_time(&s.generated_at), status), DIM);
    let gap = w.saturating_sub(left.len() + right.len()).max(2);
    fit(join(vec![left, cells(&" ".repeat(gap), PLAIN), right]), w)
}

fn clock_time(iso8601: &str) -> String {
    iso8601.chars().skip(11).take(8).collect()
}

// ╭─ Title ─────╮ for the first panel, ├─ Title ─────┤ between panels, ╰─────────╯ to close.
fn border(left: char, right: char, title: Option<&str>, w: usize) -> Row {
    let mut row = cells(&left.to_string(), DIM);
    if let Some(title) = title {
        row.extend(cells("\u{2500} ", DIM));
        row.extend(cells(title, BOLD));
        row.extend(cells(" ", DIM));
    }
    while row.len() < w - 1 {
        row.push(Cell { ch: '\u{2500}', style: DIM });
    }
    row.extend(cells(&right.to_string(), DIM));
    row
}

// │ content … gutter │ : the content gets w - 6 cells; the gutter's middle cell is the link's rail.
fn boxed(content: Row, w: usize) -> Row {
    join(vec![cells("\u{2502} ", DIM), fit(content, w - 6), cells("   ", PLAIN), cells("\u{2502}", DIM)])
}

// Out of the row it leaves to its rail, down the rail through whatever lies between, and back in
// to the row it feeds: ━ across, ┃ down, ╂ where it crosses a panel border, ╋ another link.
fn draw_link(frame: &mut [Row], from: usize, to: usize, w: usize, style: Style) {
    let rail = w - 3;
    let content_end = |row: &Row| (2..rail).rev().find(|&x| row[x].ch != ' ').unwrap_or(1);
    let put = |frame: &mut [Row], row: usize, x: usize, ch: char| {
        let joined = match (frame[row][x].ch, ch) {
            ('\u{2500}', '\u{2503}') => '\u{2542}',
            ('\u{2501}', '\u{2503}') | ('\u{2503}' | '\u{2542}' | '\u{254B}', '\u{2501}') => '\u{254B}',
            _ => ch,
        };
        frame[row][x] = Cell { ch: joined, style };
    };
    let start = content_end(&frame[from]) + 2;
    for x in start..rail {
        put(frame, from, x, '\u{2501}');
    }
    put(frame, from, rail, '\u{2513}');
    for row in from + 1..to {
        put(frame, row, rail, '\u{2503}');
    }
    let start = content_end(&frame[to]) + 2;
    if start < rail {
        put(frame, to, start, '\u{25C0}');
    }
    for x in start + 1..rail {
        put(frame, to, x, '\u{2501}');
    }
    put(frame, to, rail, '\u{251B}');
}

// Pads every column but the last to the widest cell in it, two blanks between columns.
fn table(rows: Vec<Vec<Row>>) -> Vec<Row> {
    let columns = rows.first().map_or(0, Vec::len);
    let widths: Vec<usize> = (0..columns).map(|c| rows.iter().map(|row| row[c].len()).max().unwrap_or(0)).collect();
    rows.into_iter()
        .map(|row| {
            let mut line: Row = Vec::new();
            for (c, cell) in row.into_iter().enumerate() {
                if c > 0 {
                    line.extend(cells("  ", PLAIN));
                }
                line.extend(if c == columns - 1 { cell } else { fit(cell, widths[c]) });
            }
            line
        })
        .collect()
}

fn port_rows(ports: &[Port], devices: &[UsbDevice], power: &Power) -> Vec<Row> {
    let feeding = feeding_port(ports).map(|port| port.id.as_str());
    table(
        ports
            .iter()
            .map(|port| {
                let fed = port.power_in == Some(true);
                let watts = power.adapter.as_ref().filter(|_| feeding == Some(port.id.as_str())).map(|adapter| format!(" {} W", adapter.watts));
                vec![
                    cells(&port.label, if port.active { BOLD } else { PLAIN }),
                    if port.active { cells("\u{25CF}", fg(GREEN)) } else { cells("\u{25CB}", DIM) },
                    cells(&port.transports.join(" "), fg(CYAN)),
                    cells(&device_names(port, devices).join(", "), PLAIN),
                    if fed { cells(&format!("power-in{}", watts.unwrap_or_default()), POWER) } else { Vec::new() },
                ]
            })
            .collect(),
    )
}

const GAUGE: usize = 24;

// The rows of the Power panel, and which of them names the source: where the link lands.
fn power_rows(p: &Power, feeding: Option<&Port>) -> (Vec<Row>, Option<usize>) {
    let level = if p.fully_charged || p.charging || p.percent >= 50 {
        GREEN
    } else if p.percent >= 20 {
        YELLOW
    } else {
        RED
    };
    let filled = (p.percent.clamp(0, 100) as usize * GAUGE + 50) / 100;
    let state_style = if p.fully_charged || p.charging {
        fg(GREEN)
    } else if p.external_connected {
        PLAIN
    } else {
        fg(YELLOW)
    };
    let mut rows = vec![join(vec![
        cells(&"\u{2588}".repeat(filled), fg(level)),
        cells(&"\u{2591}".repeat(GAUGE - filled), DIM),
        cells(&format!("  {}%", p.percent), strong(level)),
        cells("  ", PLAIN),
        cells(&power_state(p), state_style),
    ])];
    if let Some(flow) = &p.battery {
        let direction = if p.charging { "in" } else { "out" };
        rows.push(join(vec![
            cells("battery  ", DIM),
            cells(&format!("{} W", compact(flow.watts)), BOLD),
            cells(&format!("  \u{00B7}  {} V \u{00D7} {} A  \u{00B7}  {direction}", compact(flow.volts), compact(flow.amps)), PLAIN),
        ]));
    }
    let on = feeding.map(|port| format!("  \u{00B7}  on {}", port.label));
    let mut source_row = None;
    if let Some(a) = &p.adapter {
        source_row = Some(rows.len());
        rows.push(join(vec![
            cells("adapter  ", POWER),
            cells(&format!("{} W", a.watts), BOLD),
            cells(&format!("  \u{00B7}  {} V \u{00D7} {} A", compact(a.volts), compact(a.amps)), PLAIN),
            cells(&if a.description.is_empty() { String::new() } else { format!("  \u{00B7}  {}", a.description) }, PLAIN),
            cells(&on.unwrap_or_default(), POWER),
        ]));
    } else if p.external_connected {
        source_row = Some(rows.len());
        rows.push(join(vec![
            cells("source   ", POWER),
            cells("external power", PLAIN),
            match on {
                Some(on) => cells(&on, POWER),
                None => cells("  \u{00B7}  port unknown", DIM),
            },
        ]));
    }
    (rows, source_row)
}

/// The Devices panel's rows, and the locationId of the device each row is: where a link out of
/// this panel starts. A row that carries a network interface names it, as a fed port says so.
fn device_rows(devices: &[UsbDevice], ports: &[Port], network: &[Interface]) -> (Vec<Row>, Vec<String>) {
    let mut rows: Vec<Vec<Row>> = Vec::new();
    let mut locations: Vec<String> = Vec::new();
    for device in devices {
        let port_label = ports.iter().find(|port| port.id == device.port).map_or(device.port.clone(), |port| port.label.clone());
        rows.push(vec![cells(&label(device), BOLD), cells(&detail(device), DIM), speed_cells(device), cells(&port_label, DIM), carried_cells(device, network)]);
        locations.push(device.location_id.clone());
        branch(&device.children, "", network, &mut rows, &mut locations);
    }
    (table(rows), locations)
}

fn carried_cells(device: &UsbDevice, network: &[Interface]) -> Row {
    cells(&carried_interfaces(device, network).join(", "), NETWORK)
}

// The Network panel: one row per interface, each naming the device it rides on, where it has one.
fn network_rows(network: &[Interface], devices: &[UsbDevice]) -> Vec<Row> {
    table(
        network
            .iter()
            .map(|interface| {
                let on = carrier_name(interface, devices);
                vec![
                    cells(interface.kind.label(), NETWORK),
                    cells(&interface.name, if interface.connected { BOLD } else { PLAIN }),
                    if interface.connected { cells("\u{25CF}", fg(GREEN)) } else { cells("\u{25CB}", DIM) },
                    cells(&link_rate(interface.speed_mbps), BOLD),
                    cells(&if on.is_empty() { String::new() } else { format!("on {on}") }, NETWORK),
                ]
            })
            .collect(),
    )
}

// A hub's devices hang off it: ├─ for each but the last, └─ for the last, │ carried down.
fn branch(children: &[UsbDevice], prefix: &str, network: &[Interface], rows: &mut Vec<Vec<Row>>, locations: &mut Vec<String>) {
    for (index, child) in children.iter().enumerate() {
        let last = index == children.len() - 1;
        let twig = if last { "\u{2514}\u{2500} " } else { "\u{251C}\u{2500} " };
        rows.push(vec![join(vec![cells(&format!("{prefix}{twig}"), DIM), cells(&label(child), PLAIN)]), cells(&detail(child), DIM), speed_cells(child), Vec::new(), carried_cells(child, network)]);
        locations.push(child.location_id.clone());
        branch(&child.children, &format!("{prefix}{}", if last { "   " } else { "\u{2502}  " }), network, rows, locations);
    }
}

// Faster buses stand out: SuperSpeed green, high speed blue, the slow ones dim.
fn speed_cells(device: &UsbDevice) -> Row {
    let style = match device.speed {
        3..=5 => fg(GREEN),
        2 => fg(BLUE),
        _ => DIM,
    };
    cells(&device.speed_label, style)
}

fn display_rows(displays: &[Display]) -> Vec<Row> {
    table(
        displays
            .iter()
            .map(|d| {
                let tags: Vec<&str> = [(d.builtin, "builtin"), (d.main, "main")].iter().filter(|(on, _)| *on).map(|(_, tag)| *tag).collect();
                vec![cells(&d.name, BOLD), cells(&format!("{}x{}", d.width, d.height), fg(MAGENTA)), cells(refresh(d).trim_start_matches(" @ "), PLAIN), cells(&tags.join(" "), DIM)]
            })
            .collect(),
    )
}
