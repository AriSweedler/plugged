use super::merge::merge_containers;
use super::{compact, detail, device_names, feeding_port, label, power_state, refresh, width};
use crate::model::{Display, Port, Power, Snapshot, UsbDevice};

pub fn render_text(s: &Snapshot) -> String {
    let devices = merge_containers(&s.usb);
    let none = || vec!["  (none)".to_string()];
    let mut lines = vec!["Ports".to_string()];
    lines.extend(if s.ports.is_empty() { none() } else { table(s.ports.iter().map(|port| port_row(port, &devices, &s.power, feeding_port(&s.ports))).collect()) });
    lines.extend(["".to_string(), "USB".to_string()]);
    lines.extend(if devices.is_empty() { none() } else { table(devices.iter().flat_map(|device| usb_rows(device, &s.ports, 1)).collect()) });
    lines.extend(["".to_string(), "Displays".to_string()]);
    lines.extend(if s.displays.is_empty() { none() } else { s.displays.iter().map(display_line).collect() });
    lines.extend(["".to_string(), "Power".to_string()]);
    lines.extend(power_lines(&s.power, feeding_port(&s.ports)));
    lines.join("\n")
}

fn port_row(port: &Port, devices: &[UsbDevice], power: &Power, feeding: Option<&Port>) -> Vec<String> {
    let watts = power.adapter.as_ref().filter(|_| feeding.is_some_and(|fed| fed.id == port.id)).map(|adapter| format!(" {} W", adapter.watts));
    vec![
        format!("  {}", port.label),
        if port.active { "\u{25CF}" } else { "\u{25CB}" }.to_string(),
        if port.transports.is_empty() { "-".to_string() } else { port.transports.join(" ") },
        device_names(port, devices).join(", "),
        if port.power_in == Some(true) { format!("power-in{}", watts.unwrap_or_default()) } else { String::new() },
    ]
}

fn usb_rows(device: &UsbDevice, ports: &[Port], depth: usize) -> Vec<Vec<String>> {
    let port_label = if depth == 1 {
        ports.iter().find(|port| port.id == device.port).map_or(device.port.clone(), |port| port.label.clone())
    } else {
        String::new()
    };
    let mut rows = vec![vec![format!("{}{}", "  ".repeat(depth), label(device)), detail(device), device.speed_label.clone(), port_label]];
    rows.extend(device.children.iter().flat_map(|child| usb_rows(child, ports, depth + 1)));
    rows
}

fn display_line(d: &Display) -> String {
    let tags: Vec<&str> = [(d.builtin, "builtin"), (d.main, "main")].iter().filter(|(on, _)| *on).map(|(_, tag)| *tag).collect();
    let tail = if tags.is_empty() { String::new() } else { format!("  {}", tags.join(" ")) };
    format!("  {}  {}x{}{}{}", d.name, d.width, d.height, refresh(d), tail)
}

fn power_lines(p: &Power, feeding: Option<&Port>) -> Vec<String> {
    let mut lines = vec![format!("  battery {}%  {}", p.percent, power_state(p))];
    if let Some(flow) = &p.battery {
        lines.push(format!("  battery {} W  {} V x {} A  {}", compact(flow.watts), compact(flow.volts), compact(flow.amps), if p.charging { "in" } else { "out" }));
    }
    let on = feeding.map(|port| format!("  on {}", port.label)).unwrap_or_default();
    if let Some(a) = &p.adapter {
        lines.push(format!("  adapter {} W  {} V x {} A  {}{on}", a.watts, compact(a.volts), compact(a.amps), a.description).trim_end().to_string());
    } else if p.external_connected {
        lines.push(format!("  source  external power{}", if on.is_empty() { "  port unknown".to_string() } else { on }));
    }
    lines
}

// Pads every column but the last to the widest cell in it, then trims trailing blanks.
fn table(rows: Vec<Vec<String>>) -> Vec<String> {
    let columns = rows.first().map_or(0, Vec::len);
    let widths: Vec<usize> = (0..columns).map(|c| rows.iter().map(|row| width(&row[c])).max().unwrap_or(0)).collect();
    rows.iter()
        .map(|row| {
            let cells: Vec<String> = row.iter().enumerate().map(|(c, cell)| if c == columns - 1 { cell.clone() } else { format!("{cell:<w$}", w = widths[c]) }).collect();
            cells.join("  ").trim_end().to_string()
        })
        .collect()
}
