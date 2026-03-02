//! # libstealthtech-core
//!
//! Open-source Rust library for interfacing with Lovesac StealthTech Sound + Charge
//! systems via Bluetooth Low Energy (BLE).
//!
//! ## Overview
//!
//! StealthTech is a Harman Kardon-powered surround sound system embedded in Lovesac
//! Sactionals furniture. The center channel exposes a BLE GATT server that the official
//! Lovesac StealthTech app (iOS/Android) connects to for configuration and control.
//!
//! This library provides:
//! - **BLE Discovery**: Scan for and identify StealthTech center channels
//! - **GATT Exploration**: Enumerate all services and characteristics (for reverse engineering)
//! - **Protocol Layer**: Encode/decode known BLE commands
//! - **Device Control**: High-level API for volume, input, EQ, sound modes, and profiles
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                  User Application                    │
//! ├─────────────────────────────────────────────────────┤
//! │              device::StealthTechDevice               │  ← High-level API
//! ├─────────────────────────────────────────────────────┤
//! │           protocol::Command / Response               │  ← Protocol encoding
//! ├─────────────────────────────────────────────────────┤
//! │         ble::Connection  ←→  ble::Scanner           │  ← BLE transport
//! ├─────────────────────────────────────────────────────┤
//! │                    btleplug                          │  ← Platform BLE
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use libstealthtech_core::{Scanner, StealthTechDevice};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Scan for StealthTech devices
//!     let scanner = Scanner::new().await?;
//!     let devices = scanner.scan(std::time::Duration::from_secs(5)).await?;
//!
//!     if let Some(peripheral) = devices.first() {
//!         // Connect and control
//!         let mut device = StealthTechDevice::connect(peripheral.clone()).await?;
//!         device.set_volume(18).await?;  // 0-36 scale
//!         device.set_input(libstealthtech_core::Input::HdmiArc).await?;
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Contributing Protocol Findings
//!
//! This library's protocol layer is built through community reverse engineering.
//! Use the `stealthtech sniff` tool to discover GATT characteristics and capture
//! BLE traffic, then submit findings via GitHub issues or PRs.

pub mod ble;
pub mod device;

/// Re-export the protocol crate as `protocol` for backwards compatibility.
pub use libstealthtech_protocol as protocol;

// Re-exports for ergonomic use
pub use ble::connection::{ConnectionConfig, ConnectionState};
pub use ble::scanner::{DiscoveredDevice, Scanner};
pub use device::StealthTechDevice;
pub use protocol::{Command, ConfigShape, DeviceState, Input, ProtocolError, Response, SoundMode};
