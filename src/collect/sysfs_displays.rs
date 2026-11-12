use super::sysfs;
use crate::model::Display;

/// One directory per connector under class/drm, named card<N>-<connector>; a display is a
/// connector that is connected and enabled. Its name and native mode come from its EDID.
pub fn collect(sys: &str) -> Vec<Display> {
    let root = format!("{sys}/class/drm");
    sysfs::entries(&root)
        .into_iter()
        .filter_map(|entry| {
            let dir = format!("{root}/{entry}");
            let connector = entry.strip_prefix("card")?.split_once('-')?.1;
            if sysfs::string(&format!("{dir}/status")).as_deref() != Some("connected") || sysfs::string(&format!("{dir}/enabled")).as_deref() != Some("enabled") {
                return None;
            }
            let edid = sysfs::bytes(&format!("{dir}/edid"));
            let timing = edid_native_timing(&edid);
            let listed = listed_mode(&dir);
            let (width, height) = timing.map(|t| (t.0, t.1)).or(listed)?;
            Some(Display {
                name: edid_name(&edid).unwrap_or_else(|| connector.to_string()),
                width,
                height,
                hz: timing.map_or(0.0, |t| t.2),
                builtin: ["eDP", "LVDS", "DSI"].iter().any(|prefix| connector.starts_with(prefix)),
                // A console has no main display; the notion belongs to a desktop session.
                main: false,
            })
        })
        .collect()
}

// The first line of `modes` is the connector's preferred mode, as WIDTHxHEIGHT.
fn listed_mode(dir: &str) -> Option<(u32, u32)> {
    let modes = sysfs::string(&format!("{dir}/modes"))?;
    let (width, height) = modes.lines().next()?.split_once('x')?;
    let digits = |text: &str| text.chars().take_while(char::is_ascii_digit).collect::<String>().parse::<u32>().ok();
    Some((digits(width)?, digits(height)?)).filter(|&(w, h)| w > 0 && h > 0)
}

// EDID keeps four 18-byte descriptors from byte 54; one tagged 0xFC is the monitor's name.
fn edid_name(edid: &[u8]) -> Option<String> {
    if edid.len() < 128 {
        return None;
    }
    (54..126).step_by(18).filter(|&start| edid[start] == 0 && edid[start + 1] == 0 && edid[start + 3] == 0xFC).find_map(|start| {
        let text: Vec<u8> = edid[start + 5..start + 18].iter().copied().take_while(|&b| b != 0x0A).collect();
        let name = String::from_utf8_lossy(&text).trim().to_string();
        (!name.is_empty()).then_some(name)
    })
}

// The first descriptor is the native mode's detailed timing: a pixel clock in 10 kHz units and
// the active and blanking counts, 12 bits each, whose product is one frame.
fn edid_native_timing(edid: &[u8]) -> Option<(u32, u32, f64)> {
    if edid.len() < 72 {
        return None;
    }
    let d: Vec<u64> = edid[54..72].iter().map(|&b| b as u64).collect();
    let clock = (d[0] | d[1] << 8) * 10_000;
    let width = d[2] | (d[4] & 0xF0) << 4;
    let horizontal_blank = d[3] | (d[4] & 0x0F) << 8;
    let height = d[5] | (d[7] & 0xF0) << 4;
    let vertical_blank = d[6] | (d[7] & 0x0F) << 8;
    let frame = (width + horizontal_blank) * (height + vertical_blank);
    if clock == 0 || frame == 0 || width == 0 || height == 0 {
        return None;
    }
    Some((width as u32, height as u32, (clock as f64 / frame as f64 * 100.0).round() / 100.0))
}
