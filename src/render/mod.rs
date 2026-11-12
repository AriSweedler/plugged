//! The two views of a snapshot. Both are drawn from the snapshot alone, so a document from a
//! file or another machine renders the same as a live one.

mod frame;
mod merge;
mod text;

pub use frame::{build_frame, render_ansi, truncate};
pub use text::render_text;

use crate::model::{Display, Port, Power, UsbDevice};

pub(crate) fn device_names(port: &Port, devices: &[UsbDevice]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for device in devices.iter().filter(|device| device.port == port.id) {
        if !names.contains(&device.name) {
            names.push(device.name.clone());
        }
    }
    names
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
