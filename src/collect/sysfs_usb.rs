use super::sysfs;
use crate::model::{Storage, UsbDevice};
use crate::render::compact;

/// One directory per device under bus/usb/devices, named <bus>-<port>.<port>…; the name is
/// the topology. usbN is a root hub, the host controller itself, and N-P:C.I is an interface.
pub fn collect(sys: &str) -> Vec<UsbDevice> {
    let root = format!("{sys}/bus/usb/devices");
    let names: Vec<String> = sysfs::entries(&root).into_iter().filter(|name| name.contains('-') && !name.contains(':')).collect();
    let mut roots: Vec<UsbDevice> = names
        .iter()
        .filter(|name| !parent(name).is_some_and(|parent| names.iter().any(|other| other == parent)))
        .map(|name| build(name, &names, &root))
        .collect();
    roots.sort_by(|a, b| a.location_id.cmp(&b.location_id));
    roots
}

fn build(name: &str, names: &[String], root: &str) -> UsbDevice {
    let mut device = usb_device(&format!("{root}/{name}"));
    device.children = names.iter().filter(|other| parent(other) == Some(name)).map(|child| build(child, names, root)).collect();
    device.children.sort_by(|a, b| a.location_id.cmp(&b.location_id));
    device
}

// 1-7.4.2 hangs off 1-7.4; 1-7 hangs off the root hub, which makes it a root here.
fn parent(name: &str) -> Option<&str> {
    name.rfind('.').map(|dot| &name[..dot])
}

fn usb_device(dir: &str) -> UsbDevice {
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
        // The container id lives in the BOS descriptor, which sysfs does not publish, so a
        // USB 3 hub's two halves stay two devices here.
        container_id: None,
        port: format!("bus {bus}"),
        storage: storage(dir),
        children: Vec::new(),
    }
}

// A USB disk shows up as a block device somewhere under one of the device's interfaces
// (<bus>-<port>:<config>.<interface>/host…/block/sdX), with its size in 512-byte sectors.
// sysfs has no volume names: those are the file systems' own.
fn storage(dir: &str) -> Option<Storage> {
    fn block_size(dir: &str, depth: u8) -> Option<u64> {
        if let Some(disk) = sysfs::entries(&format!("{dir}/block")).first() {
            return sysfs::int(&format!("{dir}/block/{disk}/size")).map(|sectors| sectors as u64 * 512);
        }
        if depth == 0 {
            return None;
        }
        sysfs::entries(dir).iter().filter(|name| name.starts_with("host") || name.starts_with("target") || name.contains(':')).find_map(|name| block_size(&format!("{dir}/{name}"), depth - 1))
    }
    let interfaces = sysfs::entries(dir).into_iter().filter(|name| name.contains(':'));
    let bytes = interfaces.into_iter().find_map(|interface| block_size(&format!("{dir}/{interface}"), 6)).filter(|bytes| *bytes > 0)?;
    Some(Storage { bytes, volumes: Vec::new() })
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
