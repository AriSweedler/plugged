use super::ioreg::{self, Entry};
use crate::model::{Port, PortKind};

pub fn collect() -> Vec<Port> {
    let mut found: Vec<(u8, i64, Port)> = Vec::new();
    ioreg::for_each_service("IOPort", |entry| {
        let p = ioreg::properties(entry);
        let (Some(kind), Some(number)) = (port_kind(p.string("PortTypeDescription").as_deref()), p.int("PortNumber")) else { return };
        let mut link_rate: Option<String> = None;
        let mut power_in: Option<bool> = p.strings("FeaturesEnabled").iter().any(|feature| feature == "Power In").then_some(false);
        ioreg::for_each_child(entry, "IOService", |child| {
            let c = ioreg::properties(child);
            match c.string("TransportTypeDescription").or_else(|| c.string("FeatureTypeDescription")).as_deref() {
                Some("DisplayPort") if c.bool("Active") == Some(true) => link_rate = c.string("LinkRateDescription"),
                Some("Power In") => power_in = Some(has_live_power_contract(child)),
                _ => {}
            }
        });
        let mut transports: Vec<String> = p.strings("TransportsActive").into_iter().filter(|transport| transport != "CC").collect();
        transports.sort_by_key(|transport| transport_rank(transport));
        let port = Port {
            id: p.string("Description").unwrap_or_else(|| format!("Port-{}@{number}", kind.label())),
            kind,
            label: format!("{} {number}", kind.label()),
            active: p.bool("ConnectionActive").unwrap_or(false),
            transports,
            display_port_link_rate: link_rate,
            power_in,
        };
        found.push((if kind == PortKind::UsbC { 0 } else { 1 }, number, port));
    });
    found.sort_by_key(|(rank, number, _)| (*rank, *number));
    found.into_iter().map(|(_, _, port)| port).collect()
}

fn port_kind(description: Option<&str>) -> Option<PortKind> {
    match description {
        Some("USB-C") => Some(PortKind::UsbC),
        Some("HDMI") => Some(PortKind::Hdmi),
        _ => None,
    }
}

// CC is the configuration channel, live whenever a cable is present, so it says nothing about data.
fn transport_rank(transport: &str) -> u8 {
    match transport {
        "USB2" => 0,
        "USB3" => 1,
        "DisplayPort" => 2,
        "CIO" => 3,
        _ => 9,
    }
}

// The Power In feature lists candidate sources (USB-PD, Brick ID, TypeC) as children; the one
// actually negotiated carries WinningPowerSourceOption. An idle port has no children at all.
fn has_live_power_contract(feature: Entry) -> bool {
    let mut live = false;
    ioreg::for_each_child(feature, "IOService", |source| {
        if ioreg::properties(source).has("WinningPowerSourceOption") {
            live = true;
        }
    });
    live
}
