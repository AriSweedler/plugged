use crate::model::UsbDevice;
use std::collections::HashMap;

/// Folds siblings that share a containerId into the faster half and pools their children.
/// Children are merged after pooling because a downstream hub's two halves arrive under
/// different upstream halves and only become siblings once those are folded.
pub fn merge_containers(devices: &[UsbDevice]) -> Vec<UsbDevice> {
    let mut merged: Vec<UsbDevice> = Vec::new();
    let mut index_by_container: HashMap<&str, usize> = HashMap::new();
    for device in devices {
        match device.container_id.as_deref() {
            Some(container) if index_by_container.contains_key(container) => {
                let index = index_by_container[container];
                merged[index] = fold(&merged[index], device);
            }
            container => {
                if let Some(container) = container {
                    index_by_container.insert(container, merged.len());
                }
                merged.push(device.clone());
            }
        }
    }
    for device in &mut merged {
        device.children = merge_containers(&device.children);
    }
    merged
}

fn fold(a: &UsbDevice, b: &UsbDevice) -> UsbDevice {
    let mut kept = if a.speed >= b.speed { a.clone() } else { b.clone() };
    kept.children = a.children.iter().chain(&b.children).cloned().collect();
    kept.children.sort_by(|x, y| x.location_id.cmp(&y.location_id));
    kept
}
