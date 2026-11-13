use super::sysfs;
use super::sysfs_usb::location_id;
use crate::model::{Interface, LinkKind};
use std::collections::HashMap;
use std::path::Path;

/// class/net holds every interface, the virtual ones included: the loopback, a bridge, a veth.
/// Only an interface backed by hardware has a `device` link, and that is the one a cable can go
/// into, so the rest are skipped. `carrier` is the live link and `speed` the rate it negotiated,
/// in Mb/s, which a driver that cannot say leaves empty or reports as -1.
pub fn collect(sys: &str) -> Vec<Interface> {
    let root = format!("{sys}/class/net");
    let on_usb = usb_interfaces(sys);
    sysfs::entries(&root)
        .into_iter()
        .filter(|name| Path::new(&format!("{root}/{name}/device")).exists())
        .map(|name| {
            let dir = format!("{root}/{name}");
            Interface {
                kind: kind(&dir),
                connected: sysfs::int(&format!("{dir}/carrier")) == Some(1),
                speed_mbps: sysfs::int(&format!("{dir}/speed")).filter(|&speed| speed > 0).unwrap_or(0),
                usb_location_id: on_usb.get(&name).cloned(),
                name,
            }
        })
        .collect()
}

// A wireless interface publishes the phy it runs on; what is left goes by its ARPHRD type,
// 1 being Ethernet.
fn kind(dir: &str) -> LinkKind {
    if Path::new(&format!("{dir}/phy80211")).exists() {
        LinkKind::WiFi
    } else if sysfs::int(&format!("{dir}/type")) == Some(1) {
        LinkKind::Ethernet
    } else {
        LinkKind::Other
    }
}

/// A USB adapter's interface hangs under one of the USB device's own interfaces, at
/// <bus>-<port>:<config>.<interface>/net/<name>, the way a USB disk's block device does.
fn usb_interfaces(sys: &str) -> HashMap<String, String> {
    let root = format!("{sys}/bus/usb/devices");
    let mut found: HashMap<String, String> = HashMap::new();
    for device in sysfs::entries(&root).into_iter().filter(|name| name.contains('-') && !name.contains(':')) {
        let dir = format!("{root}/{device}");
        let location = location_id(sysfs::int(&format!("{dir}/busnum")).unwrap_or(0), &sysfs::string(&format!("{dir}/devpath")).unwrap_or_default());
        for interface in sysfs::entries(&dir).into_iter().filter(|name| name.contains(':')) {
            for name in sysfs::entries(&format!("{dir}/{interface}/net")) {
                found.insert(name, location.clone());
            }
        }
    }
    found
}
