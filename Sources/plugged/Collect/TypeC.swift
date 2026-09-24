import Foundation
import IOKit

func collectPorts() -> [Port] {
    var found: [(rank: Int, number: Int, port: Port)] = []
    ioForEachService(matching: "IOPort") { entry in
        let p = ioProperties(entry)
        guard let kind = portKind(p.string("PortTypeDescription")), let number = p.int("PortNumber") else { return }
        var linkRate: String?
        var powerIn: Bool? = p.strings("FeaturesEnabled").contains("Power In") ? false : nil
        ioForEachChild(of: entry, plane: "IOService") { child in
            let c = ioProperties(child)
            switch c.string("TransportTypeDescription") ?? c.string("FeatureTypeDescription") {
            case "DisplayPort" where c.bool("Active") == true:
                linkRate = c.string("LinkRateDescription")
            case "Power In":
                powerIn = hasLivePowerContract(child)
            default:
                break
            }
        }
        let port = Port(
            id: p.string("Description") ?? "Port-\(kind.label)@\(number)",
            kind: kind,
            label: "\(kind.label) \(number)",
            active: p.bool("ConnectionActive") ?? false,
            transports: p.strings("TransportsActive").filter { $0 != "CC" }.sorted { transportRank($0) < transportRank($1) },
            displayPortLinkRate: linkRate,
            powerIn: powerIn)
        found.append((kind == .usbC ? 0 : 1, number, port))
    }
    return found.sorted { ($0.rank, $0.number) < ($1.rank, $1.number) }.map(\.port)
}

private func portKind(_ description: String?) -> PortKind? {
    switch description {
    case "USB-C": .usbC
    case "HDMI": .hdmi
    default: nil
    }
}

// CC is the configuration channel, live whenever a cable is present, so it says nothing about data.
private func transportRank(_ transport: String) -> Int {
    switch transport {
    case "USB2": 0
    case "USB3": 1
    case "DisplayPort": 2
    case "CIO": 3
    default: 9
    }
}

// The Power In feature lists candidate sources (USB-PD, Brick ID, TypeC) as children; the one
// actually negotiated carries WinningPowerSourceOption. An idle port has no children at all.
private func hasLivePowerContract(_ feature: IOEntry) -> Bool {
    var live = false
    ioForEachChild(of: feature, plane: "IOService") { source in
        if ioProperties(source)["WinningPowerSourceOption"] != nil { live = true }
    }
    return live
}
