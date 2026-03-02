//! BLE scanner for discovering StealthTech center channel devices.
//!
//! The StealthTech center channel advertises as "StealthTech Sound + Charge"
//! or similar via BLE. This module handles platform-agnostic device discovery.

use std::time::Duration;

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use tracing::{info, warn};

use crate::protocol::characteristics::STEALTHTECH_DEVICE_NAMES;

/// BLE scanner that discovers StealthTech devices.
pub struct Scanner {
    adapter: Adapter,
}

/// A discovered StealthTech device, not yet connected.
#[derive(Debug, Clone)]
pub struct DiscoveredDevice {
    /// The underlying BLE peripheral handle.
    pub peripheral: Peripheral,
    /// The advertised device name, if available.
    pub name: Option<String>,
    /// The BLE MAC address or platform identifier.
    pub address: String,
    /// RSSI signal strength at discovery time.
    pub rssi: Option<i16>,
}

/// Return a usable identifier for a peripheral.
///
/// On macOS, CoreBluetooth does not expose real MAC addresses — btleplug
/// returns `00:00:00:00:00:00` for every device. In that case we fall back
/// to `Peripheral::id()` which gives the stable CoreBluetooth UUID.
fn peripheral_address(
    peripheral: &Peripheral,
    properties: &btleplug::api::PeripheralProperties,
) -> String {
    let addr = properties.address.to_string();
    if addr == "00:00:00:00:00:00" {
        peripheral.id().to_string()
    } else {
        addr
    }
}

impl Scanner {
    /// Create a new scanner using the first available Bluetooth adapter.
    pub async fn new() -> anyhow::Result<Self> {
        let manager = Manager::new().await?;
        let adapters = manager.adapters().await?;
        let adapter = adapters
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No Bluetooth adapters found"))?;

        let adapter_info = adapter.adapter_info().await?;
        info!(adapter = ?adapter_info, "Using Bluetooth adapter");
        Ok(Self { adapter })
    }

    /// Create a scanner with a specific adapter.
    pub fn with_adapter(adapter: Adapter) -> Self {
        Self { adapter }
    }

    /// Scan for StealthTech devices for the given duration.
    ///
    /// Returns all peripherals whose advertised name matches known StealthTech
    /// device name patterns (case-insensitive partial match).
    pub async fn scan(&self, duration: Duration) -> anyhow::Result<Vec<DiscoveredDevice>> {
        info!(?duration, "Starting BLE scan for StealthTech devices");

        self.adapter.start_scan(ScanFilter::default()).await?;
        tokio::time::sleep(duration).await;
        self.adapter.stop_scan().await?;

        let peripherals = self.adapter.peripherals().await?;
        let mut devices = Vec::new();

        for peripheral in peripherals {
            if let Ok(Some(properties)) = peripheral.properties().await {
                let name = properties.local_name.clone();
                let is_stealthtech = name.as_ref().is_some_and(|n| {
                    let lower = n.to_lowercase();
                    STEALTHTECH_DEVICE_NAMES
                        .iter()
                        .any(|pattern| lower.contains(pattern))
                });

                if is_stealthtech {
                    let device = DiscoveredDevice {
                        address: peripheral_address(&peripheral, &properties),
                        rssi: properties.rssi,
                        name,
                        peripheral,
                    };
                    info!(
                        name = ?device.name,
                        address = %device.address,
                        rssi = ?device.rssi,
                        "Found StealthTech device"
                    );
                    devices.push(device);
                }
            }
        }

        if devices.is_empty() {
            warn!("No StealthTech devices found during scan");
        }

        Ok(devices)
    }

    /// Scan for ALL BLE peripherals (useful for reverse engineering / debugging).
    /// Returns every peripheral found during the scan window.
    pub async fn scan_all(&self, duration: Duration) -> anyhow::Result<Vec<DiscoveredDevice>> {
        info!(?duration, "Starting full BLE scan (all devices)");

        self.adapter.start_scan(ScanFilter::default()).await?;
        tokio::time::sleep(duration).await;
        self.adapter.stop_scan().await?;

        let peripherals = self.adapter.peripherals().await?;
        let mut devices = Vec::new();

        for peripheral in peripherals {
            if let Ok(Some(properties)) = peripheral.properties().await {
                devices.push(DiscoveredDevice {
                    address: peripheral_address(&peripheral, &properties),
                    rssi: properties.rssi,
                    name: properties.local_name.clone(),
                    peripheral,
                });
            }
        }

        info!(count = devices.len(), "Full scan complete");
        Ok(devices)
    }

    /// Stop an active scan.
    pub async fn stop(&self) -> anyhow::Result<()> {
        self.adapter.stop_scan().await?;
        Ok(())
    }
}
