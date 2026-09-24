import Foundation

// Folds siblings that share a containerId into the faster half and pools their children.
// Children are merged after pooling because a downstream hub's two halves arrive under
// different upstream halves and only become siblings once those are folded.
func mergeContainers(_ devices: [USBDevice]) -> [USBDevice] {
    var merged: [USBDevice] = []
    var indexByContainer: [String: Int] = [:]
    for device in devices {
        if let container = device.containerId, let index = indexByContainer[container] {
            merged[index] = fold(merged[index], device)
        } else {
            if let container = device.containerId { indexByContainer[container] = merged.count }
            merged.append(device)
        }
    }
    return merged.map { device in
        var device = device
        device.children = mergeContainers(device.children)
        return device
    }
}

private func fold(_ a: USBDevice, _ b: USBDevice) -> USBDevice {
    var kept = a.speed >= b.speed ? a : b
    kept.children = (a.children + b.children).sorted { $0.locationId < $1.locationId }
    return kept
}
