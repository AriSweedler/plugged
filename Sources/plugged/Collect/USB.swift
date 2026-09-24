import Foundation
import IOKit

func collectUSB() -> [USBDevice] {
    var nodes: [UInt64: USBDevice] = [:]
    var parentOf: [UInt64: UInt64] = [:]
    ioForEachService(matching: "IOUSBHostDevice") { entry in
        let id = ioEntryID(entry)
        nodes[id] = usbDevice(entry, ioProperties(entry))
        if let parent = ioParent(entry, plane: "IOUSB") {
            parentOf[id] = ioEntryID(parent)
            IOObjectRelease(parent)
        }
    }
    return usbTree(nodes, parentOf: parentOf)
}

private func usbDevice(_ entry: IOEntry, _ p: [String: Any]) -> USBDevice {
    let location = UInt32(truncatingIfNeeded: p.int("locationID") ?? 0)
    let speed = p.int("Device Speed") ?? -1
    let bcd = p.int("bcdDevice") ?? 0
    return USBDevice(
        name: p.string("USB Product Name") ?? ioEntryName(entry),
        vendor: p.string("USB Vendor Name") ?? "",
        vendorId: p.int("idVendor") ?? 0,
        productId: p.int("idProduct") ?? 0,
        revision: String(format: "%x.%02x", bcd >> 8, bcd & 0xff),
        serial: p.string("USB Serial Number"),
        speed: speed,
        speedLabel: usbSpeedLabel(speed),
        locationId: String(format: "0x%08x", location),
        containerId: p.string("kUSBContainerID"),
        // The top byte of locationID is the host controller's bus index; bus N sits behind built-in port N+1.
        port: "Port-USB-C@\(Int(location >> 24) + 1)",
        children: [])
}

private func usbSpeedLabel(_ speed: Int) -> String {
    switch speed {
    case 0: "1.5 Mb/s"
    case 1: "12 Mb/s"
    case 2: "480 Mb/s"
    case 3: "5 Gb/s"
    case 4: "10 Gb/s"
    case 5: "20 Gb/s"
    default: "unknown"
    }
}

// A parent that is not itself a matched device is a host controller, which makes the device a root.
private func usbTree(_ nodes: [UInt64: USBDevice], parentOf: [UInt64: UInt64]) -> [USBDevice] {
    var childIDs: [UInt64: [UInt64]] = [:]
    var rootIDs: [UInt64] = []
    for id in nodes.keys {
        if let parent = parentOf[id], nodes[parent] != nil {
            childIDs[parent, default: []].append(id)
        } else {
            rootIDs.append(id)
        }
    }
    func build(_ id: UInt64) -> USBDevice? {
        guard var device = nodes[id] else { return nil }
        device.children = (childIDs[id] ?? []).compactMap(build).sorted { $0.locationId < $1.locationId }
        return device
    }
    return rootIDs.compactMap(build).sorted { $0.locationId < $1.locationId }
}
