//! The snapshot: the one document every platform's collectors fill and every view draws.
//! docs/SPEC.md says what each field means; tests/conform.jq checks it.

use serde::{Deserialize, Serialize, Serializer};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub schema_version: u32,
    pub generated_at: String,
    pub host: String,
    pub ports: Vec<Port>,
    pub usb: Vec<UsbDevice>,
    pub displays: Vec<Display>,
    /// Absent from a document written before interfaces were collected: an empty list there.
    #[serde(default)]
    pub network: Vec<Interface>,
    pub power: Power,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum PortKind {
    #[serde(rename = "usb-c")]
    UsbC,
    #[serde(rename = "hdmi")]
    Hdmi,
}

impl PortKind {
    pub fn label(self) -> &'static str {
        match self {
            PortKind::UsbC => "USB-C",
            PortKind::Hdmi => "HDMI",
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Port {
    pub id: String,
    pub kind: PortKind,
    pub label: String,
    pub active: bool,
    pub transports: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_port_link_rate: Option<String>,
    /// None when the port cannot take power in; true only while a power contract is live on it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_in: Option<bool>,
}

/// What a USB device is when it is a disk: its size, and the volumes on it by name.
#[derive(Serialize, Deserialize, Clone)]
pub struct Storage {
    pub bytes: u64,
    pub volumes: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UsbDevice {
    pub name: String,
    pub vendor: String,
    pub vendor_id: u32,
    pub product_id: u32,
    pub revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
    pub speed: i32,
    pub speed_label: String,
    pub location_id: String,
    /// A USB 3 hub enumerates as two devices, one per bus generation, that share this id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    /// The id of the Port this device hangs off, so `usb[].port == ports[].id` joins them.
    pub port: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage: Option<Storage>,
    pub children: Vec<UsbDevice>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Display {
    pub name: String,
    pub width: u32,
    pub height: u32,
    #[serde(serialize_with = "whole_as_integer")]
    pub hz: f64,
    pub builtin: bool,
    pub main: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum LinkKind {
    #[serde(rename = "ethernet")]
    Ethernet,
    #[serde(rename = "wi-fi")]
    WiFi,
    #[serde(rename = "other")]
    Other,
}

impl LinkKind {
    pub fn label(self) -> &'static str {
        match self {
            LinkKind::Ethernet => "Ethernet",
            LinkKind::WiFi => "Wi-Fi",
            LinkKind::Other => "Link",
        }
    }
}

/// A network interface the machine has in hardware, and whether a live link runs over it.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Interface {
    pub name: String,
    pub kind: LinkKind,
    /// A cable is in and the link is up, or the radio is associated.
    pub connected: bool,
    pub speed_mbps: i64,
    /// The locationId of the USB device the interface lives on, so
    /// `network[].usbLocationId == usb[].locationId` joins them; absent when it is not on USB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usb_location_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Adapter {
    pub watts: i64,
    #[serde(serialize_with = "whole_as_integer")]
    pub volts: f64,
    #[serde(serialize_with = "whole_as_integer")]
    pub amps: f64,
    pub description: String,
}

/// What flows into the battery while it charges, or out of it otherwise.
#[derive(Serialize, Deserialize, Clone)]
pub struct BatteryFlow {
    #[serde(serialize_with = "whole_as_integer")]
    pub watts: f64,
    #[serde(serialize_with = "whole_as_integer")]
    pub volts: f64,
    #[serde(serialize_with = "whole_as_integer")]
    pub amps: f64,
}

impl BatteryFlow {
    /// From millivolts and milliamps, rounded to what a person reads; None when nothing flows.
    pub fn from_milli(millivolts: i64, milliamps: i64) -> Option<BatteryFlow> {
        let round = |value: f64| (value * 100.0).round() / 100.0;
        let volts = millivolts as f64 / 1000.0;
        let amps = milliamps.abs() as f64 / 1000.0;
        (millivolts > 0 && milliamps != 0).then(|| BatteryFlow { watts: (volts * amps * 10.0).round() / 10.0, volts: round(volts), amps: round(amps) })
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Power {
    pub external_connected: bool,
    pub charging: bool,
    pub fully_charged: bool,
    pub percent: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_remaining_min: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<Adapter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub battery: Option<BatteryFlow>,
}

// 60 Hz is written 60, not 60.0, so the document reads the same from every implementation.
fn whole_as_integer<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        serializer.serialize_i64(*value as i64)
    } else {
        serializer.serialize_f64(*value)
    }
}
