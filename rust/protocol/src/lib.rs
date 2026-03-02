//! # libstealthtech-protocol
//!
//! Pure protocol types for the StealthTech BLE protocol.
//!
//! This crate contains **zero native dependencies** — no btleplug, no tokio —
//! making it suitable for compilation to WebAssembly and use in any environment.
//!
//! ## Contents
//!
//! - [`characteristics`] — GATT service/characteristic UUIDs and constants
//! - [`commands`] — Command/Response enums, encoding/decoding
//! - [`state`] — DeviceState struct for tracking device state

pub mod characteristics;
pub mod commands;
pub mod state;

pub use commands::{Command, ConfigShape, Input, ProtocolError, Response, SoundMode};
pub use state::DeviceState;
