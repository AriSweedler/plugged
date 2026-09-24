import Foundation

struct Snapshot: Codable {
    var schemaVersion = 1
    let generatedAt: String
    let host: String
    let ports: [Port]
    let usb: [USBDevice]
    let displays: [Display]
    let power: Power
}

enum PortKind: String, Codable {
    case usbC = "usb-c"
    case hdmi

    var label: String {
        switch self {
        case .usbC: "USB-C"
        case .hdmi: "HDMI"
        }
    }
}

struct Port: Codable {
    let id: String
    let kind: PortKind
    let label: String
    let active: Bool
    let transports: [String]
    let displayPortLinkRate: String?
    // nil when the port cannot take power in; true only while a power contract is live on it.
    let powerIn: Bool?
}

struct USBDevice: Codable {
    let name: String
    let vendor: String
    let vendorId: Int
    let productId: Int
    let revision: String
    let serial: String?
    let speed: Int
    let speedLabel: String
    let locationId: String
    // A USB 3 hub enumerates as two devices, one per bus generation, that share this id.
    let containerId: String?
    // The id of the Port this device hangs off, so `usb[].port == ports[].id` joins them.
    let port: String
    var children: [USBDevice]
}

struct Display: Codable {
    let name: String
    let width: Int
    let height: Int
    let hz: Double
    let builtin: Bool
    let main: Bool
}

struct Adapter: Codable {
    let watts: Int
    let volts: Double
    let amps: Double
    let description: String
}

struct Power: Codable {
    let externalConnected: Bool
    let charging: Bool
    let fullyCharged: Bool
    let percent: Int
    let timeRemainingMin: Int?
    let adapter: Adapter?
}
