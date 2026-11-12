//! The two views of a snapshot. Both are drawn from the snapshot alone, so a document from a
//! file or another machine renders the same as a live one.

mod frame;
mod merge;
mod text;

pub use frame::{build_frame, render_ansi, render_plain};
pub use text::render_text;

use crate::model::{Display, Port, Power, UsbDevice};

pub(crate) fn device_names(port: &Port, devices: &[UsbDevice]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for device in devices.iter().filter(|device| device.port == port.id) {
        let name = label(device);
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// What a person calls the device: a disk goes by its volumes, since an enclosure's own
/// product string is as plain as "External"; anything else by its product name.
pub(crate) fn label(device: &UsbDevice) -> String {
    match &device.storage {
        Some(storage) if !storage.volumes.is_empty() => storage.volumes.join(", "),
        _ => device.name.clone(),
    }
}

/// What the device says it is, beside its label: maker, product when the label is not it, size.
pub(crate) fn detail(device: &UsbDevice) -> String {
    let mut parts: Vec<String> = Vec::new();
    let named_by_volumes = label(device) != device.name;
    if !device.vendor.is_empty() && !device.name.to_lowercase().contains(&device.vendor.to_lowercase()) && !device.vendor.to_lowercase().contains(&device.name.to_lowercase()) {
        parts.push(if named_by_volumes { format!("{} {}", device.vendor, device.name) } else { device.vendor.clone() });
    } else if named_by_volumes {
        parts.push(device.name.clone());
    }
    if let Some(storage) = &device.storage {
        parts.push(size(storage.bytes));
    }
    parts.join(" \u{00B7} ")
}

/// Bytes in the decimal units a disk is sold in: 2 TB, 500 GB, 64 GB.
pub(crate) fn size(bytes: u64) -> String {
    let units = ["B", "kB", "MB", "GB", "TB", "PB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 999.5 && unit < units.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    let rounded = if value >= 100.0 { value.round() } else if value >= 10.0 { (value * 10.0).round() / 10.0 } else { (value * 100.0).round() / 100.0 };
    format!("{} {}", compact(rounded), units[unit])
}

/// The port that feeds the machine: the first with a live power contract. A platform that
/// reports no ports has none, and the views say the port is unknown.
pub(crate) fn feeding_port(ports: &[Port]) -> Option<&Port> {
    ports.iter().find(|port| port.power_in == Some(true))
}

pub(crate) fn power_state(p: &Power) -> String {
    let mut state = if p.fully_charged {
        "full"
    } else if p.charging {
        "charging"
    } else if p.external_connected {
        "external power, not charging"
    } else {
        "on battery"
    }
    .to_string();
    if let Some(minutes) = p.time_remaining_min {
        state += &if p.charging { format!(", {minutes} min to full") } else { format!(", {minutes} min left") };
    }
    state
}

/// A platform that cannot tell the refresh rate reports 0, which is left out.
pub(crate) fn refresh(d: &Display) -> String {
    if d.hz > 0.0 {
        format!(" @ {} Hz", compact(d.hz))
    } else {
        String::new()
    }
}

/// A number as a person writes it: six significant digits at most, no trailing zeros.
pub fn compact(value: f64) -> String {
    if value == 0.0 || !value.is_finite() {
        return "0".to_string();
    }
    let decimals = (5 - value.abs().log10().floor() as i32).clamp(0, 12) as usize;
    let text = format!("{value:.decimals$}");
    if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        text
    }
}

pub(crate) fn width(text: &str) -> usize {
    text.chars().count()
}
