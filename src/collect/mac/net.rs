use super::ioreg::{self, Props};
use crate::model::{Interface, LinkKind};

// How far above an interface its radio or its USB device sits: the interface's controller is
// the first ancestor, and a driver puts a few entries of its own between that and the hardware.
const ANCESTORS: usize = 8;
// The classes a Wi-Fi radio's driver stack is made of, the kernel's and DriverKit's.
const WIFI_CLASSES: [&str; 4] = ["IO80211Interface", "IO80211Controller", "IO80211SkywalkInterface", "IOUserNetworkWLAN"];

/// Every IONetworkInterface is backed by a driver, so the loopback, a bridge and a tunnel are
/// never among them. The link is on the interface's controller, its parent: `IOLinkStatus` has
/// bit 0 set when the status is known and bit 1 when the link is up, and `IOLinkSpeed` is the
/// rate in bits a second. An interface the system keeps for itself (the Mac's own ports as a USB
/// device) is marked `HiddenInterface` or `HiddenConfiguration` and left out.
pub fn collect() -> Vec<Interface> {
    let mut found: Vec<Interface> = Vec::new();
    ioreg::for_each_service("IONetworkInterface", |entry| {
        let interface = ioreg::properties(entry);
        let Some(name) = interface.string("BSD Name") else { return };
        let mut controller: Option<Props> = None;
        let mut wifi = ioreg::conforms_to(entry, "IO80211Interface");
        let mut usb_location_id: Option<String> = None;
        ioreg::for_each_ancestor(entry, "IOService", ANCESTORS, |ancestor| {
            if controller.is_none() {
                controller = Some(ioreg::properties(ancestor));
            }
            wifi = wifi || WIFI_CLASSES.iter().any(|class| ioreg::conforms_to(ancestor, class));
            if ioreg::conforms_to(ancestor, "IOUSBHostDevice") {
                // The same number, written the same way, as the device's locationId in the USB tree.
                usb_location_id = ioreg::properties(ancestor).int("locationID").map(|location| format!("0x{:08x}", location as u32));
            }
            !wifi && usb_location_id.is_none()
        });
        let controller = controller.unwrap_or_default();
        if hidden(&interface) || hidden(&controller) {
            return;
        }
        let connected = controller.int("IOLinkStatus").is_some_and(|status| status & 0b11 == 0b11);
        found.push(Interface {
            kind: kind(wifi, &interface),
            connected,
            // A link that is down negotiated nothing, whatever rate the port could reach.
            speed_mbps: if connected { controller.int("IOLinkSpeed").unwrap_or(0).max(0) / 1_000_000 } else { 0 },
            usb_location_id,
            name,
        });
    });
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

fn hidden(p: &Props) -> bool {
    p.bool("HiddenInterface") == Some(true) || p.bool("HiddenConfiguration") == Some(true)
}

// What is not a radio goes by its `IOInterfaceType`, 6 being Ethernet (IFT_ETHER).
fn kind(wifi: bool, interface: &Props) -> LinkKind {
    if wifi {
        LinkKind::WiFi
    } else if interface.int("IOInterfaceType") == Some(6) {
        LinkKind::Ethernet
    } else {
        LinkKind::Other
    }
}
