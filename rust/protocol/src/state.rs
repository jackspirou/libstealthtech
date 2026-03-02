//! StealthTech device state tracking.
//!
//! Fields map to MCU state variables confirmed by firmware analysis and
//! the homebridge-lovesac-stealthtech protocol implementation.

use serde::{Deserialize, Serialize};

use crate::characteristics::{
    FirmwareComponentVersion, LATEST_DSP_VERSION, LATEST_EQ_VERSION, LATEST_MCU_VERSION,
};
use crate::commands::{ConfigShape, Input, Response, SoundMode};

/// Last known state of the StealthTech system.
///
/// Updated locally when commands are sent and when notifications are received.
/// Fields are `Option` because we may not know the state until we read it
/// from the device (request via `Command::GetState`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceState {
    // === Audio Controls ===
    /// Current volume level (0-36). MCU: `gSys.VolLevel`.
    pub volume: Option<u8>,
    /// Current input source. MCU: `gSys.Source_State`.
    pub input: Option<Input>,
    /// Current sound preset. MCU: `gSys.SystemMode`.
    pub sound_mode: Option<SoundMode>,
    /// Whether the system is muted. MCU: `gSys.MuteState`.
    pub mute: Option<bool>,

    // === EQ / Balance ===
    /// Bass level (0-20). MCU: `AppDate.BassVal`.
    pub bass: Option<u8>,
    /// Treble level (0-20). MCU: `AppDate.TrebleVal`.
    pub treble: Option<u8>,
    /// Center channel volume offset (0-30). MCU: `AppDate.Center_vol`.
    pub center_volume: Option<u8>,
    /// L/R balance (0-100, 50=center). MCU: `AppDate.BlanceVal`.
    pub balance: Option<u8>,
    /// Rear channel volume (0-30). MCU: `AppDate.RearChannelVol`.
    pub rear_channel_volume: Option<u8>,

    // === Speaker Configuration ===
    /// Whether surround speakers are enabled.
    pub surround_enabled: Option<bool>,
    /// Whether Quiet Couch Mode is active. MCU: `AppDate.QuiteMode`.
    pub quiet_couch: Option<bool>,
    /// Whether the subwoofer is connected.
    pub subwoofer_connected: Option<bool>,

    // === Tuning ===
    /// Fabric covering type ID. MCU: `AppDate.CovingVal`.
    pub fabric: Option<u8>,
    /// Configuration shape for surround calibration. MCU: `AppDate.SystemLayoutVal`.
    pub config_shape: Option<ConfigShape>,
    /// Arm type. MCU: `AppDate.ArmType`.
    pub arm_type: Option<u8>,

    // === System ===
    /// Whether the device is powered on. MCU: `gSys.Power_status`.
    /// Note: in the BLE protocol, power is inverted (0=ON, 1=OFF).
    pub power: Option<bool>,
    /// MCU firmware version (fw_type=1).
    pub mcu_version: Option<FirmwareComponentVersion>,
    /// DSP firmware version (fw_type=2).
    pub dsp_version: Option<FirmwareComponentVersion>,
    /// EQ firmware version (fw_type=3).
    pub eq_version: Option<FirmwareComponentVersion>,
    /// Legacy single firmware version string (kept for backward compat).
    /// This is overwritten by each firmware notification; prefer the
    /// per-component fields above.
    pub firmware_version: Option<String>,
    /// Model number from Device Information Service.
    pub model_number: Option<String>,
    /// Manufacturer name from Device Information Service.
    pub manufacturer: Option<String>,
}

impl DeviceState {
    /// Update a single field from a decoded BLE notification response.
    ///
    /// Call this for each `Response` decoded from the notification stream
    /// to keep the local state in sync with the device.
    pub fn apply_response(&mut self, response: &Response) {
        match response {
            Response::Volume(v) => self.volume = Some(*v),
            Response::CenterVolume(v) => self.center_volume = Some(*v),
            Response::Treble(v) => self.treble = Some(*v),
            Response::Bass(v) => self.bass = Some(*v),
            Response::MuteState(v) => self.mute = Some(*v),
            Response::QuietMode(v) => self.quiet_couch = Some(*v),
            Response::Balance(v) => self.balance = Some(*v),
            Response::Layout(v) => self.config_shape = ConfigShape::from_byte(*v).ok(),
            Response::CurrentInput(v) => self.input = Some(*v),
            Response::Power(v) => self.power = Some(*v),
            Response::CurrentSoundMode(v) => self.sound_mode = Some(*v),
            Response::Covering(v) => self.fabric = Some(*v),
            Response::ArmType(v) => self.arm_type = Some(*v),
            Response::SubwooferConnected(v) => self.subwoofer_connected = Some(*v),
            Response::RearVolume(v) => self.rear_channel_volume = Some(*v),
            Response::FirmwareVersion {
                fw_type,
                major,
                minor,
            } => {
                let ver = FirmwareComponentVersion::new(*major, *minor);
                match fw_type {
                    1 => self.mcu_version = Some(ver),
                    2 => self.dsp_version = Some(ver),
                    3 => self.eq_version = Some(ver),
                    _ => {}
                }
                let name = match fw_type {
                    1 => "MCU",
                    2 => "DSP",
                    3 => "EQ",
                    _ => "Unknown",
                };
                self.firmware_version = Some(format!("{} v{}.{}", name, major, minor));
            }
            Response::Unknown { .. } => {}
        }
    }

    /// Returns true if any firmware component is older than the latest known version.
    ///
    /// Returns `None` if no firmware version info has been received yet.
    pub fn firmware_update_available(&self) -> Option<bool> {
        // Need at least one component to make a determination
        if self.mcu_version.is_none() && self.dsp_version.is_none() && self.eq_version.is_none() {
            return None;
        }

        let outdated = self
            .mcu_version
            .is_some_and(|v| !v.is_at_least(&LATEST_MCU_VERSION))
            || self
                .dsp_version
                .is_some_and(|v| !v.is_at_least(&LATEST_DSP_VERSION))
            || self
                .eq_version
                .is_some_and(|v| !v.is_at_least(&LATEST_EQ_VERSION));

        Some(outdated)
    }
}
