use super::ioreg::{self, Props};
use crate::model::{Adapter, Power};

pub fn collect() -> Power {
    // A Mac with no battery runs from the wall; this is what it reports if AppleSmartBattery is absent.
    let mut power = Power { external_connected: true, charging: false, fully_charged: false, percent: 0, time_remaining_min: None, adapter: None };
    ioreg::for_each_service("AppleSmartBattery", |entry| {
        let p = ioreg::properties(entry);
        power = Power {
            external_connected: p.bool("ExternalConnected").unwrap_or(false),
            charging: p.bool("IsCharging").unwrap_or(false),
            fully_charged: p.bool("FullyCharged").unwrap_or(false),
            percent: percent(p.int("CurrentCapacity"), p.int("MaxCapacity")),
            time_remaining_min: p.int("TimeRemaining").filter(|&minutes| minutes > 0 && minutes != 65535), // 65535: still estimating
            adapter: p.dict("AdapterDetails").and_then(adapter),
        };
    });
    power
}

// Apple silicon reports capacity as a 0-100 percentage; Intel Macs report mAh against MaxCapacity.
fn percent(current: Option<i64>, max: Option<i64>) -> i64 {
    match (current, max) {
        (Some(current), Some(max)) if max > 0 => (current * 100 / max).clamp(0, 100),
        _ => 0,
    }
}

fn adapter(d: &Props) -> Option<Adapter> {
    let millivolts = d.int("AdapterVoltage").unwrap_or(0);
    let milliamps = d.int("Current").unwrap_or(0);
    let watts = d.int("Watts").unwrap_or((millivolts * milliamps + 500_000) / 1_000_000);
    (watts > 0).then(|| Adapter {
        watts,
        volts: millivolts as f64 / 1000.0,
        amps: milliamps as f64 / 1000.0,
        description: d.string("Name").or_else(|| d.string("Description")).unwrap_or_default(),
    })
}
