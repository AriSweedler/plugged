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
    // Only a platform that reports ports names them.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
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

#[derive(Serialize, Deserialize, Clone)]
pub struct Adapter {
    pub watts: i64,
    #[serde(serialize_with = "whole_as_integer")]
    pub volts: f64,
    #[serde(serialize_with = "whole_as_integer")]
    pub amps: f64,
    pub description: String,
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
}

// 60 Hz is written 60, not 60.0, so the document reads the same from every implementation.
fn whole_as_integer<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        serializer.serialize_i64(*value as i64)
    } else {
        serializer.serialize_f64(*value)
    }
}
