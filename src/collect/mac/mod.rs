//! macOS: the IORegistry through IOKit, and the screens through AppKit. No subprocess:
//! `system_profiler` returns no USB devices on some builds, and parsing `ioreg` is what this
//! replaces.

mod displays;
mod ioreg;
mod ports;
mod power;
mod usb;

use crate::model::{Snapshot, SCHEMA_VERSION};

pub fn collect(generated_at: String, host: String) -> Snapshot {
    Snapshot {
        schema_version: SCHEMA_VERSION,
        generated_at,
        host,
        ports: ports::collect(),
        usb: usb::collect(),
        displays: displays::collect(),
        // Not read here yet: the registry's IOEthernetInterface side is unwritten (docs/SPEC.md).
        network: Vec::new(),
        power: power::collect(),
    }
}
