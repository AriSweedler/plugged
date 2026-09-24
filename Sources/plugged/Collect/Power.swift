import Foundation
import IOKit

func collectPower() -> Power {
    // A Mac with no battery runs from the wall; this is what it reports if AppleSmartBattery is absent.
    var power = Power(externalConnected: true, charging: false, fullyCharged: false, percent: 0, timeRemainingMin: nil, adapter: nil)
    ioForEachService(matching: "AppleSmartBattery") { entry in
        let p = ioProperties(entry)
        power = Power(
            externalConnected: p.bool("ExternalConnected") ?? false,
            charging: p.bool("IsCharging") ?? false,
            fullyCharged: p.bool("FullyCharged") ?? false,
            percent: percent(current: p.int("CurrentCapacity"), max: p.int("MaxCapacity")),
            timeRemainingMin: timeRemaining(p.int("TimeRemaining")),
            adapter: adapter(p.dict("AdapterDetails")))
    }
    return power
}

// Apple silicon reports capacity as a 0-100 percentage; Intel Macs report mAh against MaxCapacity.
private func percent(current: Int?, max: Int?) -> Int {
    guard let current, let max, max > 0 else { return 0 }
    return current * 100 / max
}

// 65535 is the controller's "still estimating" sentinel.
private func timeRemaining(_ minutes: Int?) -> Int? {
    guard let minutes, minutes > 0, minutes != 65535 else { return nil }
    return minutes
}

private func adapter(_ d: [String: Any]?) -> Adapter? {
    guard let d else { return nil }
    let millivolts = d.int("AdapterVoltage") ?? 0
    let milliamps = d.int("Current") ?? 0
    let watts = d.int("Watts") ?? (millivolts * milliamps + 500_000) / 1_000_000
    guard watts > 0 else { return nil }
    return Adapter(
        watts: watts,
        volts: Double(millivolts) / 1000,
        amps: Double(milliamps) / 1000,
        description: d.string("Name") ?? d.string("Description") ?? "")
}
