//! GATT service and characteristic discovery for reverse engineering.
//!
//! This module provides tools to enumerate all BLE GATT services and characteristics
//! exposed by the StealthTech center channel, read their values, and log everything
//! in a structured format for protocol analysis.

use std::collections::BTreeSet;

use btleplug::api::{Characteristic, CharPropFlags};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::ble::connection::Connection;

/// A discovered GATT service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    pub uuid: String,
    pub is_primary: bool,
    pub characteristics: Vec<DiscoveredCharacteristic>,
}

/// A discovered GATT characteristic with its properties and current value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredCharacteristic {
    pub uuid: String,
    pub service_uuid: String,
    pub properties: Vec<String>,
    pub value_hex: Option<String>,
    pub value_utf8: Option<String>,
    pub is_known: bool,
    pub description: Option<String>,
}

/// Full GATT profile dump of a StealthTech device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GattProfile {
    pub device_name: Option<String>,
    pub device_address: String,
    pub timestamp: String,
    pub services: Vec<DiscoveredService>,
}

/// Maps standard BLE characteristic UUIDs to human-readable names.
fn known_characteristic_name(uuid: &str) -> Option<&'static str> {
    match uuid.to_lowercase().as_str() {
        // Standard BLE services/characteristics
        "00002a00-0000-1000-8000-00805f9b34fb" => Some("Device Name"),
        "00002a01-0000-1000-8000-00805f9b34fb" => Some("Appearance"),
        "00002a04-0000-1000-8000-00805f9b34fb" => Some("Peripheral Preferred Connection Parameters"),
        "00002a24-0000-1000-8000-00805f9b34fb" => Some("Model Number String"),
        "00002a25-0000-1000-8000-00805f9b34fb" => Some("Serial Number String"),
        "00002a26-0000-1000-8000-00805f9b34fb" => Some("Firmware Revision String"),
        "00002a27-0000-1000-8000-00805f9b34fb" => Some("Hardware Revision String"),
        "00002a28-0000-1000-8000-00805f9b34fb" => Some("Software Revision String"),
        "00002a29-0000-1000-8000-00805f9b34fb" => Some("Manufacturer Name String"),
        "0000180a-0000-1000-8000-00805f9b34fb" => Some("[Service] Device Information"),
        "00001800-0000-1000-8000-00805f9b34fb" => Some("[Service] Generic Access"),
        "00001801-0000-1000-8000-00805f9b34fb" => Some("[Service] Generic Attribute"),
        _ => None,
    }
}

/// Discover and dump the complete GATT profile of a connected StealthTech device.
///
/// This is the primary reverse engineering tool. It enumerates every service and
/// characteristic, reads all readable values, and produces a structured report.
pub async fn discover_gatt_profile(
    connection: &mut Connection,
    device_name: Option<String>,
    device_address: String,
) -> anyhow::Result<GattProfile> {
    info!("Starting GATT profile discovery...");

    let characteristics: BTreeSet<Characteristic> = connection.characteristics();
    info!(
        count = characteristics.len(),
        "Found GATT characteristics"
    );

    // Group characteristics by service UUID
    let mut services_map: std::collections::BTreeMap<String, Vec<DiscoveredCharacteristic>> =
        std::collections::BTreeMap::new();

    for char in &characteristics {
        let uuid_str = char.uuid.to_string();
        let service_uuid_str = char.service_uuid.to_string();

        // Decode properties
        let mut props = Vec::new();
        if char.properties.contains(CharPropFlags::READ) {
            props.push("READ".to_string());
        }
        if char.properties.contains(CharPropFlags::WRITE) {
            props.push("WRITE".to_string());
        }
        if char.properties.contains(CharPropFlags::WRITE_WITHOUT_RESPONSE) {
            props.push("WRITE_NO_RESP".to_string());
        }
        if char.properties.contains(CharPropFlags::NOTIFY) {
            props.push("NOTIFY".to_string());
        }
        if char.properties.contains(CharPropFlags::INDICATE) {
            props.push("INDICATE".to_string());
        }

        // Try to read the value if readable
        let (value_hex, value_utf8) = if char.properties.contains(CharPropFlags::READ) {
            match connection.read(char).await {
                Ok(data) => {
                    let hex_str = hex::encode(&data);
                    let utf8_str = String::from_utf8(data.clone())
                        .ok()
                        .filter(|s| s.chars().all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace()));
                    debug!(uuid = %uuid_str, hex = %hex_str, "Read characteristic");
                    (Some(hex_str), utf8_str)
                }
                Err(e) => {
                    warn!(uuid = %uuid_str, error = %e, "Failed to read characteristic");
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

        let known_name = known_characteristic_name(&uuid_str);

        let discovered = DiscoveredCharacteristic {
            uuid: uuid_str,
            service_uuid: service_uuid_str.clone(),
            properties: props,
            value_hex,
            value_utf8,
            is_known: known_name.is_some(),
            description: known_name.map(String::from),
        };

        services_map
            .entry(service_uuid_str)
            .or_default()
            .push(discovered);
    }

    let services: Vec<DiscoveredService> = services_map
        .into_iter()
        .map(|(uuid, characteristics)| DiscoveredService {
            uuid,
            is_primary: true, // btleplug doesn't distinguish; assume primary
            characteristics,
        })
        .collect();

    let profile = GattProfile {
        device_name,
        device_address,
        timestamp: chrono::Utc::now().to_rfc3339(),
        services,
    };

    info!(
        services = profile.services.len(),
        characteristics = profile
            .services
            .iter()
            .map(|s| s.characteristics.len())
            .sum::<usize>(),
        "GATT profile discovery complete"
    );

    Ok(profile)
}

/// Print a GATT profile in a human-readable format to stdout.
pub fn print_gatt_profile(profile: &GattProfile) {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║          StealthTech GATT Profile Discovery                 ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!(
        "║ Device:  {:<51} ║",
        profile.device_name.as_deref().unwrap_or("Unknown")
    );
    println!("║ Address: {:<51} ║", profile.device_address);
    println!("║ Time:    {:<51} ║", profile.timestamp);
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    for service in &profile.services {
        let service_label = known_characteristic_name(&service.uuid)
            .unwrap_or("Unknown/Custom Service");
        println!("┌─ Service: {} ─────────", service.uuid);
        println!("│  Type: {}", service_label);
        println!("│  Characteristics: {}", service.characteristics.len());
        println!("│");

        for (i, char) in service.characteristics.iter().enumerate() {
            let is_last = i == service.characteristics.len() - 1;
            let prefix = if is_last { "└" } else { "├" };
            let cont = if is_last { " " } else { "│" };

            let desc = char
                .description
                .as_deref()
                .unwrap_or(if char.is_known { "Standard" } else { "⚠ UNKNOWN - needs mapping" });

            println!("{}── Characteristic: {}", prefix, char.uuid);
            println!("{}   Properties: [{}]", cont, char.properties.join(", "));
            println!("{}   Description: {}", cont, desc);

            if let Some(ref hex_val) = char.value_hex {
                println!("{}   Value (hex): {}", cont, hex_val);
            }
            if let Some(ref utf8_val) = char.value_utf8 {
                println!("{}   Value (utf8): \"{}\"", cont, utf8_val);
            }
            println!("{}   ", cont);
        }
        println!();
    }
}
