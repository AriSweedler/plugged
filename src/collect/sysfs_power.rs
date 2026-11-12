use super::sysfs;
use crate::model::{Adapter, BatteryFlow, Power};

/// class/power_supply holds the battery, the mains adapter and any USB power source, each
/// with a `type`. A peripheral's own battery (a mouse, a phone) says scope=Device and is skipped.
pub fn collect(sys: &str) -> Power {
    let root = format!("{sys}/class/power_supply");
    let mut battery: Option<String> = None;
    let mut external_connected = false;
    let mut adapter: Option<Adapter> = None;
    let mut saw_supply = false;
    for name in sysfs::entries(&root) {
        let dir = format!("{root}/{name}");
        if sysfs::string(&format!("{dir}/scope")).as_deref() == Some("Device") {
            continue;
        }
        match sysfs::string(&format!("{dir}/type")) {
            Some(kind) if kind == "Battery" => {
                battery.get_or_insert(dir);
            }
            Some(kind) => {
                saw_supply = true;
                if sysfs::int(&format!("{dir}/online")).unwrap_or(0) > 0 {
                    external_connected = true;
                    adapter = adapter.or_else(|| negotiated(&dir, &kind));
                }
            }
            None => {}
        }
    }
    let Some(battery) = battery else {
        // A machine with no battery runs from the wall, whether or not it names a supply.
        return Power { external_connected: external_connected || !saw_supply, charging: false, fully_charged: false, percent: 0, time_remaining_min: None, adapter, battery: None };
    };
    let status = sysfs::string(&format!("{battery}/status")).unwrap_or_default();
    Power {
        external_connected,
        charging: status == "Charging",
        fully_charged: status == "Full",
        percent: sysfs::int(&format!("{battery}/capacity")).unwrap_or(0).clamp(0, 100),
        time_remaining_min: if status == "Discharging" { minutes_left(&battery) } else { None },
        adapter,
        // Microvolts and microamps; some drivers sign the current, the status says which way.
        battery: BatteryFlow::from_milli(sysfs::int(&format!("{battery}/voltage_now")).unwrap_or(0) / 1000, sysfs::int(&format!("{battery}/current_now")).unwrap_or(0) / 1000),
    }
}

// Charge over current, or energy over power, whichever pair the battery's driver publishes.
fn minutes_left(battery: &str) -> Option<i64> {
    [("charge_now", "current_now"), ("energy_now", "power_now")].iter().find_map(|(stored, rate)| {
        let stored = sysfs::int(&format!("{battery}/{stored}"))?;
        let rate = sysfs::int(&format!("{battery}/{rate}"))?;
        (rate > 0 && stored * 60 / rate > 0).then(|| stored * 60 / rate)
    })
}

// Only a source that publishes its contract (a USB-PD port's supply) has watts to report; a
// plain Mains supply says it is online and nothing more.
fn negotiated(dir: &str, kind: &str) -> Option<Adapter> {
    let microvolts = sysfs::int(&format!("{dir}/voltage_now")).filter(|&v| v > 0)?;
    let microamps = sysfs::int(&format!("{dir}/current_max")).or_else(|| sysfs::int(&format!("{dir}/current_now"))).filter(|&a| a > 0)?;
    let volts = microvolts as f64 / 1_000_000.0;
    let amps = microamps as f64 / 1_000_000.0;
    let watts = (volts * amps).round() as i64;
    (watts > 0).then(|| Adapter { watts, volts, amps, description: sysfs::string(&format!("{dir}/usb_type")).unwrap_or_else(|| kind.to_string()) })
}
