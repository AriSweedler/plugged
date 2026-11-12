use super::ioreg::{self, Entry, Props};
use crate::model::{Storage, UsbDevice};
use std::collections::HashMap;

pub fn collect() -> Vec<UsbDevice> {
    let mut nodes: HashMap<u64, UsbDevice> = HashMap::new();
    let mut parent_of: HashMap<u64, u64> = HashMap::new();
    ioreg::for_each_service("IOUSBHostDevice", |entry| {
        let id = ioreg::entry_id(entry);
        nodes.insert(id, usb_device(entry, &ioreg::properties(entry)));
        if let Some(parent) = ioreg::parent_id(entry, "IOUSB") {
            parent_of.insert(id, parent);
        }
    });
    usb_tree(&nodes, &parent_of)
}

fn usb_device(entry: Entry, p: &Props) -> UsbDevice {
    let location = p.int("locationID").unwrap_or(0) as u32;
    let speed = p.int("Device Speed").unwrap_or(-1) as i32;
    let bcd = p.int("bcdDevice").unwrap_or(0);
    UsbDevice {
        name: p.string("USB Product Name").unwrap_or_else(|| ioreg::entry_name(entry)),
        vendor: p.string("USB Vendor Name").unwrap_or_default(),
        vendor_id: p.int("idVendor").unwrap_or(0) as u32,
        product_id: p.int("idProduct").unwrap_or(0) as u32,
        revision: format!("{:x}.{:02x}", bcd >> 8, bcd & 0xff),
        serial: p.string("USB Serial Number"),
        speed,
        speed_label: speed_label(speed).to_string(),
        location_id: format!("0x{location:08x}"),
        container_id: p.string("kUSBContainerID"),
        // The top byte of locationID is the host controller's bus index; bus N sits behind built-in port N+1.
        port: format!("Port-USB-C@{}", (location >> 24) + 1),
        storage: storage(entry),
        children: Vec::new(),
    }
}

// A disk hangs below its USB device in the service plane: the whole medium first, then its
// partitions and, for APFS, the volumes, each a registry entry whose name is the volume's.
fn storage(device: Entry) -> Option<Storage> {
    const HIDDEN_ROLES: [&str; 7] = ["Preboot", "Recovery", "VM", "xART", "Hardware", "Update", "Prelogin"];
    fn walk(entry: Entry, depth: u8, bytes: &mut Option<u64>, volumes: &mut Vec<String>) {
        if depth == 0 {
            return;
        }
        ioreg::for_each_child(entry, "IOService", |child| {
            let p = ioreg::properties(child);
            if bytes.is_none() && p.bool("Whole") == Some(true) {
                *bytes = p.int("Size").map(|size| size as u64);
            }
            if let Some(name) = p.string("FullName") {
                if !p.strings("Role").iter().any(|role| HIDDEN_ROLES.contains(&role.as_str())) && !volumes.contains(&name) {
                    volumes.push(name);
                }
            }
            walk(child, depth - 1, bytes, volumes);
        });
    }
    let (mut bytes, mut volumes) = (None, Vec::new());
    walk(device, 24, &mut bytes, &mut volumes);
    bytes.filter(|bytes| *bytes > 0).map(|bytes| Storage { bytes, volumes })
}

fn speed_label(speed: i32) -> &'static str {
    match speed {
        0 => "1.5 Mb/s",
        1 => "12 Mb/s",
        2 => "480 Mb/s",
        3 => "5 Gb/s",
        4 => "10 Gb/s",
        5 => "20 Gb/s",
        _ => "unknown",
    }
}

// A parent that is not itself a matched device is a host controller, which makes the device a root.
fn usb_tree(nodes: &HashMap<u64, UsbDevice>, parent_of: &HashMap<u64, u64>) -> Vec<UsbDevice> {
    let mut child_ids: HashMap<u64, Vec<u64>> = HashMap::new();
    let mut root_ids: Vec<u64> = Vec::new();
    for id in nodes.keys() {
        match parent_of.get(id).filter(|parent| nodes.contains_key(parent)) {
            Some(parent) => child_ids.entry(*parent).or_default().push(*id),
            None => root_ids.push(*id),
        }
    }
    fn build(id: u64, nodes: &HashMap<u64, UsbDevice>, child_ids: &HashMap<u64, Vec<u64>>) -> UsbDevice {
        let mut device = nodes[&id].clone();
        device.children = child_ids.get(&id).map(|ids| ids.iter().map(|child| build(*child, nodes, child_ids)).collect()).unwrap_or_default();
        device.children.sort_by(|a, b| a.location_id.cmp(&b.location_id));
        device
    }
    let mut roots: Vec<UsbDevice> = root_ids.into_iter().map(|id| build(id, nodes, &child_ids)).collect();
    roots.sort_by(|a, b| a.location_id.cmp(&b.location_id));
    roots
}
