use super::sysfs_smc::Smc;
use crate::model::{Port, PortKind};

/// The kernel's typec class is the usual per-port view, and an Intel Mac publishes nothing
/// there: its ports are known only through the SMC, and only for power. So a port here is
/// `active` when a source is on it and carries no transports; a machine whose SMC cannot be
/// read reports no ports at all.
pub fn collect(smc: Option<&Smc>) -> Vec<Port> {
    let Some(smc) = smc else { return Vec::new() };
    let (ports, contracts) = smc.contracts();
    let feeding = smc.feeding(&contracts).map(|contract| contract.port);
    (1..=ports)
        .map(|number| Port {
            id: format!("Port-USB-C@{number}"),
            kind: PortKind::UsbC,
            label: format!("{} {number}", PortKind::UsbC.label()),
            active: contracts.iter().any(|contract| contract.port == number),
            transports: Vec::new(),
            display_port_link_rate: None,
            power_in: Some(feeding == Some(number)),
        })
        .collect()
}
