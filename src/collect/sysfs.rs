//! Linux has no IORegistry; the kernel publishes the same facts as one small file per attribute
//! under /sys. Every sysfs collector takes that root, so a recorded tree (tests/fixtures) drives
//! the same code on any machine.

use std::fs;

pub fn entries(directory: &str) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .map(|listing| listing.flatten().filter_map(|entry| entry.file_name().into_string().ok()).collect())
        .unwrap_or_default();
    names.sort();
    names
}

pub fn string(path: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub fn int(path: &str) -> Option<i64> {
    string(path)?.parse().ok()
}

pub fn hex(path: &str) -> Option<u32> {
    u32::from_str_radix(&string(path)?, 16).ok()
}

/// The name of what a symlink chain ends at: a device's directory is named after the device.
pub fn link_name(path: &str) -> Option<String> {
    fs::canonicalize(path).ok()?.file_name()?.to_str().map(str::to_string)
}

pub fn bytes(path: &str) -> Vec<u8> {
    fs::read(path).unwrap_or_default()
}
