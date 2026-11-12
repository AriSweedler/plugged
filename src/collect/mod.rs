//! One collector set per platform, each filling the same model (docs/SPEC.md).

#[cfg(target_os = "macos")]
mod mac;
mod sysfs;
mod sysfs_displays;
mod sysfs_ports;
mod sysfs_power;
mod sysfs_smc;
mod sysfs_usb;

use crate::model::{Snapshot, SCHEMA_VERSION};

/// PLUGGED_SYSFS_ROOT points the sysfs set at a recorded tree, on any platform: that is how
/// tests/ runs the Linux collectors on a Mac.
pub fn collect_snapshot() -> Snapshot {
    let recorded = std::env::var("PLUGGED_SYSFS_ROOT").ok();
    #[cfg(target_os = "macos")]
    if recorded.is_none() {
        return mac::collect(timestamp(), host_name());
    }
    let sys = recorded.unwrap_or_else(|| "/sys".to_string());
    let smc = sysfs_smc::Smc::open(&sys);
    Snapshot {
        schema_version: SCHEMA_VERSION,
        generated_at: timestamp(),
        host: host_name(),
        ports: sysfs_ports::collect(smc.as_ref()),
        usb: sysfs_usb::collect(&sys),
        displays: sysfs_displays::collect(&sys),
        power: sysfs_power::collect(&sys, smc.as_ref()),
    }
}

// Local time with its zone, 2026-11-12T13:40:00-08:00; the standard library has no local clock.
fn timestamp() -> String {
    let mut buffer = [0u8; 40];
    let written = unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut parts: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut parts);
        libc::strftime(buffer.as_mut_ptr().cast(), buffer.len(), c"%Y-%m-%dT%H:%M:%S%z".as_ptr(), &parts)
    };
    let stamp = String::from_utf8_lossy(&buffer[..written]).into_owned();
    // strftime writes the zone as -0800; ISO 8601 wants -08:00, and Z for UTC.
    match stamp.len() {
        24 if stamp.ends_with("+0000") => format!("{}Z", &stamp[..19]),
        24 => format!("{}:{}", &stamp[..22], &stamp[22..]),
        _ => stamp,
    }
}

// gethostname never touches DNS.
fn host_name() -> String {
    let mut buffer = [0u8; 256];
    if unsafe { libc::gethostname(buffer.as_mut_ptr().cast(), buffer.len()) } != 0 {
        return String::new();
    }
    let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
    String::from_utf8_lossy(&buffer[..end]).into_owned()
}
