use super::sysfs;
use crate::model::{Storage, UsbDevice};
use crate::render::compact;
use std::collections::HashMap;

/// One directory per device under bus/usb/devices, named <bus>-<port>.<port>…; the name is
/// the topology. usbN is a root hub, the host controller itself, and N-P:C.I is an interface.
pub fn collect(sys: &str) -> Vec<UsbDevice> {
    let root = format!("{sys}/bus/usb/devices");
    let udev = udev_data(sys);
    let names: Vec<String> = sysfs::entries(&root).into_iter().filter(|name| name.contains('-') && !name.contains(':')).collect();
    let containers = containers(&names, &root);
    let mut roots: Vec<UsbDevice> = names
        .iter()
        .filter(|name| !parent(name).is_some_and(|parent| names.iter().any(|other| other == parent)))
        .map(|name| build(name, &names, &root, &udev, &containers))
        .collect();
    roots.sort_by(|a, b| a.location_id.cmp(&b.location_id));
    roots
}

// The BOS descriptor that holds a container id is not in sysfs, but the kernel links each USB 2
// port to its USB 3 twin (<device>/port is the port a device is on, <port>/peer the twin,
// <port>/device what is on it). A USB 3 hub enumerates once on each, so two devices on peered
// ports are its two halves, and both get one id made of their names.
fn containers(names: &[String], root: &str) -> HashMap<String, String> {
    names
        .iter()
        .filter_map(|name| {
            let peer = sysfs::link_name(&format!("{root}/{name}/port/peer/device")).filter(|peer| names.contains(peer))?;
            let mut pair = [name.as_str(), peer.as_str()];
            pair.sort();
            Some((name.clone(), pair.join("+")))
        })
        .collect()
}

fn build(name: &str, names: &[String], root: &str, udev: &str, containers: &HashMap<String, String>) -> UsbDevice {
    let mut device = usb_device(&format!("{root}/{name}"), udev);
    device.container_id = containers.get(name).cloned();
    device.children = names.iter().filter(|other| parent(other) == Some(name)).map(|child| build(child, names, root, udev, containers)).collect();
    device.children.sort_by(|a, b| a.location_id.cmp(&b.location_id));
    device
}

// 1-7.4.2 hangs off 1-7.4; 1-7 hangs off the root hub, which makes it a root here.
fn parent(name: &str) -> Option<&str> {
    name.rfind('.').map(|dot| &name[..dot])
}

fn usb_device(dir: &str, udev: &str) -> UsbDevice {
    let vendor_id = sysfs::hex(&format!("{dir}/idVendor")).unwrap_or(0);
    let product_id = sysfs::hex(&format!("{dir}/idProduct")).unwrap_or(0);
    let bcd = sysfs::hex(&format!("{dir}/bcdDevice")).unwrap_or(0);
    let megabits = sysfs::string(&format!("{dir}/speed")).unwrap_or_default();
    let bus = sysfs::int(&format!("{dir}/busnum")).unwrap_or(0);
    UsbDevice {
        name: sysfs::string(&format!("{dir}/product")).unwrap_or_else(|| format!("{vendor_id:04x}:{product_id:04x}")),
        vendor: sysfs::string(&format!("{dir}/manufacturer")).unwrap_or_default(),
        vendor_id,
        product_id,
        revision: format!("{:x}.{:02x}", bcd >> 8, bcd & 0xff),
        serial: sysfs::string(&format!("{dir}/serial")),
        speed: speed_code(&megabits),
        speed_label: speed_label(&megabits),
        location_id: location_id(bus, &sysfs::string(&format!("{dir}/devpath")).unwrap_or_default()),
        container_id: None,
        port: format!("bus {bus}"),
        storage: storage(dir, udev),
        children: Vec::new(),
    }
}

// A USB disk shows up as a block device somewhere under one of the device's interfaces
// (<bus>-<port>:<config>.<interface>/host…/block/sdX), with its size in 512-byte sectors.
fn storage(dir: &str, udev: &str) -> Option<Storage> {
    fn block_disk(dir: &str, depth: u8) -> Option<String> {
        if let Some(disk) = sysfs::entries(&format!("{dir}/block")).first() {
            return Some(format!("{dir}/block/{disk}"));
        }
        if depth == 0 {
            return None;
        }
        sysfs::entries(dir).iter().filter(|name| name.starts_with("host") || name.starts_with("target") || name.contains(':')).find_map(|name| block_disk(&format!("{dir}/{name}"), depth - 1))
    }
    let interfaces = sysfs::entries(dir).into_iter().filter(|name| name.contains(':'));
    let disk = interfaces.into_iter().find_map(|interface| block_disk(&format!("{dir}/{interface}"), 6))?;
    let bytes = sysfs::int(&format!("{disk}/size")).map(|sectors| sectors as u64 * 512).filter(|bytes| *bytes > 0)?;
    Some(Storage { bytes, volumes: volumes(&disk, udev) })
}

// sysfs has no volume names: those are the file systems' own, which udev reads with blkid and
// keeps in /run/udev/data/b<major>:<minor> as ID_FS_LABEL_ENC. The udev database sits beside
// /sys, so a recorded tree keeps it beside its sys/ too.
fn udev_data(sys: &str) -> String {
    let beside = std::path::Path::new(sys).parent().unwrap_or(std::path::Path::new("/"));
    beside.join("run/udev/data").to_string_lossy().into_owned()
}

// The disk's partitions (block/sdX/sdXN, each with a `partition` file) in partition order, or
// the disk itself when its file system covers it whole. An EFI system partition is left out,
// as a Mac leaves it out.
fn volumes(disk: &str, udev: &str) -> Vec<String> {
    const EFI_SYSTEM: &str = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";
    let mut partitions: Vec<(i64, String)> = sysfs::entries(disk)
        .into_iter()
        .filter_map(|name| sysfs::int(&format!("{disk}/{name}/partition")).map(|number| (number, format!("{disk}/{name}"))))
        .collect();
    partitions.sort();
    let blocks: Vec<String> = if partitions.is_empty() { vec![disk.to_string()] } else { partitions.into_iter().map(|(_, block)| block).collect() };
    blocks
        .iter()
        .filter_map(|block| sysfs::string(&format!("{block}/dev")))
        .filter_map(|dev| sysfs::string(&format!("{udev}/b{dev}")))
        .filter_map(|record| {
            let property = |key: &str| record.lines().find_map(|line| line.strip_prefix("E:")?.strip_prefix(key)?.strip_prefix('=').map(str::to_string));
            if property("ID_PART_ENTRY_TYPE").is_some_and(|kind| kind.eq_ignore_ascii_case(EFI_SYSTEM)) {
                return None;
            }
            property("ID_FS_LABEL_ENC").map(|label| unescape(&label)).or_else(|| property("ID_FS_LABEL")).filter(|label| !label.is_empty())
        })
        .collect()
}

// udev writes a label's unsafe bytes, a space among them, as \xNN.
fn unescape(label: &str) -> String {
    let raw = label.as_bytes();
    let mut bytes = Vec::with_capacity(raw.len());
    let mut at = 0;
    while at < raw.len() {
        let code = raw.get(at + 2..at + 4).and_then(|hex| u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok());
        match code {
            Some(byte) if raw[at] == b'\\' && raw[at + 1] == b'x' => {
                bytes.push(byte);
                at += 4;
            }
            _ => {
                bytes.push(raw[at]);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

// The codes macOS reports as Device Speed, so `speed` compares across machines.
fn speed_code(megabits: &str) -> i32 {
    match megabits {
        "1.5" => 0,
        "12" => 1,
        "480" => 2,
        "5000" => 3,
        "10000" => 4,
        "20000" => 5,
        _ => -1,
    }
}

fn speed_label(megabits: &str) -> String {
    match megabits.parse::<f64>() {
        Ok(value) if value >= 1000.0 => format!("{} Gb/s", compact(value / 1000.0)),
        Ok(value) => format!("{} Mb/s", compact(value)),
        Err(_) => "unknown".to_string(),
    }
}

// The shape of a Mac's locationID: the bus in the top byte, then one nibble per hub port.
pub fn location_id(bus: i64, path: &str) -> String {
    let mut location = (bus as u32) << 24;
    let mut shift = 20;
    for port in path.split('.').filter_map(|port| port.parse::<u32>().ok()).take(6) {
        location |= (port & 0xf) << shift;
        shift = if shift >= 4 { shift - 4 } else { 0 };
    }
    format!("0x{location:08x}")
}
