//! Command encoding and decoding for the StealthTech BLE protocol.
//!
//! # Protocol Architecture
//!
//! The center channel's QCC3008 BLE SoC exposes a custom GATT service with
//! the UUID `65786365-6c70-6f69-6e74-2e636f6d0000` ("excelpoint.com").
//! Commands are written to specific characteristics as short byte packets.
//! Responses arrive as notifications on the UpStream characteristic (0001).
//!
//! # Packet Formats
//!
//! **Format A** (5 bytes): `AA <cmd_id> <sub_cmd_id> 01 <value>`
//! Used for EQ, audio path, and player control commands.
//!
//! **Format B** (4 bytes): `AA <cmd_id> <value> 00`
//! Used for preset, source, and device info commands.
//!
//! # Notification Format
//!
//! Notifications on UpStream: `CC 05/06 AA ... <response_code> <value>`
//! The last 2 bytes are always the response code and value.
//!
//! # Reference
//!
//! Protocol confirmed by [homebridge-lovesac-stealthtech](https://github.com/ohmantics/homebridge-lovesac-stealthtech)
//! (MIT, Alex Rosenberg) and MCU firmware string analysis.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::characteristics::*;

/// Errors that can occur during command encoding/decoding.
#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Unknown byte value: 0x{0:02x}")]
    UnknownByte(u8),

    #[error("Invalid payload length: expected {expected}, got {actual}")]
    InvalidLength { expected: usize, actual: usize },

    #[error("Value out of range: {value} not in [{min}, {max}]")]
    OutOfRange { value: i32, min: i32, max: i32 },
}

// ============================================================================
// Packet format helpers
// ============================================================================

/// Format A: `AA <cmd_id> <sub_cmd_id> 01 <value>` (5 bytes).
fn format_a(cmd_id: u8, sub_cmd_id: u8, value: u8) -> Vec<u8> {
    vec![0xAA, cmd_id, sub_cmd_id, 0x01, value]
}

/// Format B: `AA <cmd_id> <value> 00` (4 bytes).
fn format_b(cmd_id: u8, value: u8) -> Vec<u8> {
    vec![0xAA, cmd_id, value, 0x00]
}

// ============================================================================
// Enums
// ============================================================================

/// Audio input source selection.
///
/// MCU variable: `gSys.Source_State`.
/// Written to the Source characteristic (0007) using Format B with cmd_id=0x07.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    /// HDMI ARC from TV.
    HdmiArc = 0,
    /// Bluetooth A2DP streaming.
    Bluetooth = 1,
    /// 3.5mm auxiliary input.
    Aux = 2,
    /// Optical TOSLINK input.
    Optical = 3,
}

impl Input {
    pub fn to_byte(self) -> u8 {
        self as u8
    }

    pub fn from_byte(byte: u8) -> Result<Self, ProtocolError> {
        match byte {
            0 => Ok(Input::HdmiArc),
            1 => Ok(Input::Bluetooth),
            2 => Ok(Input::Aux),
            3 => Ok(Input::Optical),
            other => Err(ProtocolError::UnknownByte(other)),
        }
    }
}

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Input::HdmiArc => write!(f, "HDMI ARC"),
            Input::Bluetooth => write!(f, "Bluetooth"),
            Input::Aux => write!(f, "AUX"),
            Input::Optical => write!(f, "Optical"),
        }
    }
}

/// Sound preset / processing mode.
///
/// MCU variable: `gSys.SystemMode`.
///
/// **Important**: Write values and read values differ!
/// - Write values (to device): Movies=7, Music=8, TV=5, News=6, Manual=9
/// - Read values (from notifications): Movies=0, Music=1, TV=2, News=3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoundMode {
    /// Optimized for movies with enhanced surround and bass.
    Movies,
    /// Optimized for music playback.
    Music,
    /// Balanced mode for TV shows.
    Tv,
    /// Enhanced dialog clarity.
    News,
    /// Manual EQ (user-defined bass/treble/balance). Write-only preset.
    Manual,
}

impl SoundMode {
    /// Encode to the **write** byte value (sent to device).
    pub fn to_write_byte(self) -> u8 {
        match self {
            SoundMode::Tv => 5,
            SoundMode::News => 6,
            SoundMode::Movies => 7,
            SoundMode::Music => 8,
            SoundMode::Manual => 9,
        }
    }

    /// Decode from the **read** byte value (received in notifications).
    /// Note: Manual mode (write value 9) has no known read value.
    pub fn from_read_byte(byte: u8) -> Result<Self, ProtocolError> {
        match byte {
            0 => Ok(SoundMode::Movies),
            1 => Ok(SoundMode::Music),
            2 => Ok(SoundMode::Tv),
            3 => Ok(SoundMode::News),
            other => Err(ProtocolError::UnknownByte(other)),
        }
    }
}

impl fmt::Display for SoundMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SoundMode::Movies => write!(f, "Movies"),
            SoundMode::Music => write!(f, "Music"),
            SoundMode::Tv => write!(f, "TV"),
            SoundMode::News => write!(f, "News"),
            SoundMode::Manual => write!(f, "Manual"),
        }
    }
}

/// Sactionals configuration shape for surround calibration.
///
/// MCU variable: `AppDate.SystemLayoutVal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigShape {
    /// Straight/linear couch layout (byte 0).
    Straight = 0,
    /// L-shaped couch layout (byte 1).
    LShape = 1,
    /// U-shaped couch layout (byte 2).
    UShape = 2,
    /// Pit/enclosed couch layout (byte 3).
    Pit = 3,
}

impl ConfigShape {
    pub fn to_byte(self) -> u8 {
        self as u8
    }

    pub fn from_byte(byte: u8) -> Result<Self, ProtocolError> {
        match byte {
            0 => Ok(ConfigShape::Straight),
            1 => Ok(ConfigShape::LShape),
            2 => Ok(ConfigShape::UShape),
            3 => Ok(ConfigShape::Pit),
            other => Err(ProtocolError::UnknownByte(other)),
        }
    }
}

impl fmt::Display for ConfigShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigShape::Straight => write!(f, "Straight"),
            ConfigShape::LShape => write!(f, "L-Shape"),
            ConfigShape::UShape => write!(f, "U-Shape"),
            ConfigShape::Pit => write!(f, "Pit"),
        }
    }
}

// ============================================================================
// Commands
// ============================================================================

/// A command to send to the StealthTech center channel.
///
/// Each command encodes to a specific characteristic UUID and byte payload.
#[derive(Debug, Clone)]
pub enum Command {
    // === Audio Controls (EqControl characteristic, 0003) ===
    /// Set master volume (0-36).
    SetVolume(u8),
    /// Set bass level (0-20).
    SetBass(u8),
    /// Set treble level (0-20).
    SetTreble(u8),
    /// Set center channel volume offset (0-30).
    SetCenterVolume(u8),
    /// Set rear channel volume (0-30).
    SetRearChannelVolume(u8),
    /// Mute (true) or unmute (false).
    SetMute(bool),
    /// Toggle Quiet Couch Mode (true=on, false=off).
    SetQuietCouch(bool),
    /// Set sound preset/mode.
    SetSoundMode(SoundMode),

    // === Audio Path (AudioPath characteristic, 0004) ===
    /// Set L/R speaker balance (0-100, 50=center).
    SetBalance(u8),
    /// Power on (true) or standby (false).
    SetPower(bool),

    // === Input (Source characteristic, 0007) ===
    /// Select audio input source.
    SetInput(Input),

    // === Tuning ===
    /// Set fabric covering type for acoustic tuning (byte ID).
    SetFabric(u8),
    /// Set configuration shape for surround calibration.
    SetConfigShape(ConfigShape),
    /// Set arm type (byte ID).
    SetArmType(u8),
    /// Enable/disable surround speakers.
    SetSurroundEnabled(bool),

    // === Media (PlayerControl characteristic, 0005) ===
    /// Play/pause Bluetooth media.
    SetPlayPause(u8),
    /// Skip forward/backward Bluetooth media.
    SetSkip(u8),

    // === System (DeviceInfo characteristic, 0002) ===
    /// Request full device state dump.
    GetState,
    /// Request firmware version info.
    GetFirmwareVersion,
}

impl Command {
    /// Validate command parameters are within protocol-defined ranges.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        match self {
            Command::SetVolume(v) if *v > MAX_VOLUME => Err(ProtocolError::OutOfRange {
                value: *v as i32,
                min: 0,
                max: MAX_VOLUME as i32,
            }),
            Command::SetBass(v) if *v > MAX_BASS => Err(ProtocolError::OutOfRange {
                value: *v as i32,
                min: 0,
                max: MAX_BASS as i32,
            }),
            Command::SetTreble(v) if *v > MAX_TREBLE => Err(ProtocolError::OutOfRange {
                value: *v as i32,
                min: 0,
                max: MAX_TREBLE as i32,
            }),
            Command::SetCenterVolume(v) if *v > MAX_CENTER_VOLUME => {
                Err(ProtocolError::OutOfRange {
                    value: *v as i32,
                    min: 0,
                    max: MAX_CENTER_VOLUME as i32,
                })
            }
            Command::SetRearChannelVolume(v) if *v > MAX_REAR_VOLUME => {
                Err(ProtocolError::OutOfRange {
                    value: *v as i32,
                    min: 0,
                    max: MAX_REAR_VOLUME as i32,
                })
            }
            Command::SetBalance(v) if *v > MAX_BALANCE => Err(ProtocolError::OutOfRange {
                value: *v as i32,
                min: 0,
                max: MAX_BALANCE as i32,
            }),
            _ => Ok(()),
        }
    }

    /// Encode command to bytes for BLE transmission.
    ///
    /// Returns the target characteristic UUID and the byte payload.
    pub fn encode(&self) -> Result<(Uuid, Vec<u8>), ProtocolError> {
        self.validate()?;

        let (uuid, data) = match self {
            // EqControl characteristic (0x03 cmd_id, Format A)
            Command::SetVolume(v) => (CHAR_EQ_CONTROL, format_a(0x03, 0x02, *v)),
            Command::SetBass(v) => (CHAR_EQ_CONTROL, format_a(0x03, 0x01, *v)),
            Command::SetTreble(v) => (CHAR_EQ_CONTROL, format_a(0x03, 0x00, *v)),
            Command::SetCenterVolume(v) => (CHAR_EQ_CONTROL, format_a(0x03, 0x03, *v)),
            Command::SetRearChannelVolume(v) => (CHAR_EQ_CONTROL, format_a(0x03, 0x0A, *v)),
            Command::SetMute(on) => (CHAR_EQ_CONTROL, format_a(0x03, 0x09, u8::from(*on))),
            Command::SetQuietCouch(on) => (CHAR_EQ_CONTROL, format_a(0x03, 0x04, u8::from(*on))),
            // Preset uses Format B
            Command::SetSoundMode(mode) => (CHAR_EQ_CONTROL, format_b(0x03, mode.to_write_byte())),

            // AudioPath characteristic (0x04 cmd_id, Format A)
            Command::SetBalance(v) => (CHAR_AUDIO_PATH, format_a(0x04, 0x00, *v)),
            Command::SetPower(on) => (CHAR_AUDIO_PATH, format_a(0x04, 0x01, u8::from(*on))),

            // Source characteristic (0x07 cmd_id, Format B)
            Command::SetInput(input) => (CHAR_SOURCE, format_b(0x07, input.to_byte())),

            // SystemLayout characteristic
            Command::SetConfigShape(shape) => (CHAR_SYSTEM_LAYOUT, format_b(0x06, shape.to_byte())),

            // Covering characteristic
            Command::SetFabric(id) => (CHAR_COVERING, format_b(0x08, *id)),

            // These commands' exact encoding is not yet confirmed
            Command::SetArmType(v) => (CHAR_EQ_CONTROL, format_a(0x03, 0x0D, *v)),
            Command::SetSurroundEnabled(on) => {
                (CHAR_EQ_CONTROL, format_a(0x03, 0x0E, u8::from(*on)))
            }

            // PlayerControl characteristic (0x05 cmd_id, Format A)
            Command::SetPlayPause(v) => (CHAR_PLAYER_CONTROL, format_a(0x05, 0x00, *v)),
            Command::SetSkip(v) => (CHAR_PLAYER_CONTROL, format_a(0x05, 0x01, *v)),

            // DeviceInfo characteristic (Format B)
            Command::GetState => (CHAR_DEVICE_INFO, format_b(0x01, 0x01)),
            // Version request differs from Format B: last byte is 0x01 not 0x00.
            // This distinguishes it from GetState (which uses format_b, trailing 0x00).
            Command::GetFirmwareVersion => (CHAR_DEVICE_INFO, vec![0xAA, 0x01, 0x01, 0x01]),
        };

        Ok((uuid, data))
    }
}

// ============================================================================
// Responses
// ============================================================================

/// A response/notification received from the StealthTech center channel.
///
/// Notifications arrive on the UpStream characteristic (0001).
/// Format: `CC 05/06 AA ... <response_code> <value>`
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Volume(u8),
    CenterVolume(u8),
    Treble(u8),
    Bass(u8),
    MuteState(bool),
    QuietMode(bool),
    Balance(u8),
    Layout(u8),
    CurrentInput(Input),
    /// Power state. Note: in the protocol, 0=ON and 1=OFF (inverted).
    Power(bool),
    CurrentSoundMode(SoundMode),
    Covering(u8),
    ArmType(u8),
    SubwooferConnected(bool),
    RearVolume(u8),
    /// Firmware version: (type, major, minor). Type: 1=MCU, 2=DSP, 3=EQ.
    FirmwareVersion {
        fw_type: u8,
        major: u8,
        minor: u8,
    },
    /// Unknown/unparseable notification data.
    Unknown {
        characteristic_uuid: Uuid,
        data: Vec<u8>,
    },
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Response::Volume(v) => write!(f, "Volume: {}/{}", v, MAX_VOLUME),
            Response::CenterVolume(v) => write!(f, "Center: {}/{}", v, MAX_CENTER_VOLUME),
            Response::Treble(v) => write!(f, "Treble: {}/{}", v, MAX_TREBLE),
            Response::Bass(v) => write!(f, "Bass: {}/{}", v, MAX_BASS),
            Response::MuteState(on) => write!(f, "Mute: {}", if *on { "on" } else { "off" }),
            Response::QuietMode(on) => write!(f, "Quiet Mode: {}", if *on { "on" } else { "off" }),
            Response::Balance(v) => write!(f, "Balance: {}/{}", v, MAX_BALANCE),
            Response::Layout(v) => write!(f, "Layout: {}", v),
            Response::CurrentInput(input) => write!(f, "Input: {}", input),
            Response::Power(on) => write!(f, "Power: {}", if *on { "on" } else { "standby" }),
            Response::CurrentSoundMode(mode) => write!(f, "Sound Mode: {}", mode),
            Response::Covering(v) => write!(f, "Covering: {}", v),
            Response::ArmType(v) => write!(f, "Arm Type: {}", v),
            Response::SubwooferConnected(on) => {
                write!(
                    f,
                    "Subwoofer: {}",
                    if *on { "connected" } else { "disconnected" }
                )
            }
            Response::RearVolume(v) => write!(f, "Rear: {}/{}", v, MAX_REAR_VOLUME),
            Response::FirmwareVersion {
                fw_type,
                major,
                minor,
            } => {
                let name = match fw_type {
                    1 => "MCU",
                    2 => "DSP",
                    3 => "EQ",
                    _ => "Unknown",
                };
                write!(f, "Firmware {}: v{}.{}", name, major, minor)
            }
            Response::Unknown {
                characteristic_uuid,
                data,
            } => {
                write!(
                    f,
                    "Unknown [{}]: {}",
                    characteristic_uuid,
                    hex::encode(data)
                )
            }
        }
    }
}

impl Response {
    /// Decode a BLE notification from the UpStream characteristic.
    ///
    /// Parses the last 2 bytes as `<response_code> <value>` for standard
    /// status responses. Version responses (`AA 01 03 ...`) are detected
    /// and parsed separately.
    pub fn decode(characteristic_uuid: Uuid, data: &[u8]) -> Self {
        if data.len() < 4 {
            return Response::Unknown {
                characteristic_uuid,
                data: data.to_vec(),
            };
        }

        // Version responses: CC 05/06 AA 01 03 <type> <major> <minor>
        if data.len() >= 8 && data[2] == 0xAA && data[3] == 0x01 && data[4] == 0x03 {
            return Response::FirmwareVersion {
                fw_type: data[5],
                major: data[6],
                minor: data[7],
            };
        }

        // Standard status: last 2 bytes are code + value
        let code = data[data.len() - 2];
        let value = data[data.len() - 1];

        if !(response_code::MIN..=response_code::MAX).contains(&code) {
            return Response::Unknown {
                characteristic_uuid,
                data: data.to_vec(),
            };
        }

        match code {
            response_code::VOLUME => Response::Volume(value),
            response_code::CENTER_VOLUME => Response::CenterVolume(value),
            response_code::TREBLE => Response::Treble(value),
            response_code::BASS => Response::Bass(value),
            response_code::MUTE => Response::MuteState(value == 1),
            response_code::QUIET_MODE => Response::QuietMode(value == 1),
            response_code::BALANCE => Response::Balance(value),
            response_code::LAYOUT => Response::Layout(value),
            response_code::SOURCE => Input::from_byte(value).map_or_else(
                |_| Response::Unknown {
                    characteristic_uuid,
                    data: data.to_vec(),
                },
                Response::CurrentInput,
            ),
            // Power is INVERTED: 0x00 = ON, 0x01 = OFF
            response_code::POWER => Response::Power(value == 0),
            response_code::PRESET => SoundMode::from_read_byte(value).map_or_else(
                |_| Response::Unknown {
                    characteristic_uuid,
                    data: data.to_vec(),
                },
                Response::CurrentSoundMode,
            ),
            response_code::COVERING => Response::Covering(value),
            response_code::ARM_TYPE => Response::ArmType(value),
            response_code::SUBWOOFER => Response::SubwooferConnected(value == 1),
            response_code::REAR_VOLUME => Response::RearVolume(value),
            _ => Response::Unknown {
                characteristic_uuid,
                data: data.to_vec(),
            },
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Command encoding tests ---

    #[test]
    fn encode_set_volume() {
        let (uuid, data) = Command::SetVolume(18).encode().unwrap();
        assert_eq!(uuid, CHAR_EQ_CONTROL);
        assert_eq!(data, vec![0xAA, 0x03, 0x02, 0x01, 18]);
    }

    #[test]
    fn encode_set_bass() {
        let (uuid, data) = Command::SetBass(10).encode().unwrap();
        assert_eq!(uuid, CHAR_EQ_CONTROL);
        assert_eq!(data, vec![0xAA, 0x03, 0x01, 0x01, 10]);
    }

    #[test]
    fn encode_set_treble() {
        let (uuid, data) = Command::SetTreble(15).encode().unwrap();
        assert_eq!(uuid, CHAR_EQ_CONTROL);
        assert_eq!(data, vec![0xAA, 0x03, 0x00, 0x01, 15]);
    }

    #[test]
    fn encode_set_center_volume() {
        let (uuid, data) = Command::SetCenterVolume(20).encode().unwrap();
        assert_eq!(uuid, CHAR_EQ_CONTROL);
        assert_eq!(data, vec![0xAA, 0x03, 0x03, 0x01, 20]);
    }

    #[test]
    fn encode_set_rear_volume() {
        let (uuid, data) = Command::SetRearChannelVolume(25).encode().unwrap();
        assert_eq!(uuid, CHAR_EQ_CONTROL);
        assert_eq!(data, vec![0xAA, 0x03, 0x0A, 0x01, 25]);
    }

    #[test]
    fn encode_set_mute() {
        let (uuid, data) = Command::SetMute(true).encode().unwrap();
        assert_eq!(uuid, CHAR_EQ_CONTROL);
        assert_eq!(data, vec![0xAA, 0x03, 0x09, 0x01, 1]);

        let (_, data) = Command::SetMute(false).encode().unwrap();
        assert_eq!(data, vec![0xAA, 0x03, 0x09, 0x01, 0]);
    }

    #[test]
    fn encode_set_quiet_mode() {
        let (uuid, data) = Command::SetQuietCouch(true).encode().unwrap();
        assert_eq!(uuid, CHAR_EQ_CONTROL);
        assert_eq!(data, vec![0xAA, 0x03, 0x04, 0x01, 1]);
    }

    #[test]
    fn encode_set_preset_movies() {
        let (uuid, data) = Command::SetSoundMode(SoundMode::Movies).encode().unwrap();
        assert_eq!(uuid, CHAR_EQ_CONTROL);
        assert_eq!(data, vec![0xAA, 0x03, 7, 0x00]); // Format B, write value 7
    }

    #[test]
    fn encode_set_preset_tv() {
        let (_, data) = Command::SetSoundMode(SoundMode::Tv).encode().unwrap();
        assert_eq!(data, vec![0xAA, 0x03, 5, 0x00]);
    }

    #[test]
    fn encode_set_preset_music() {
        let (_, data) = Command::SetSoundMode(SoundMode::Music).encode().unwrap();
        assert_eq!(data, vec![0xAA, 0x03, 8, 0x00]);
    }

    #[test]
    fn encode_set_preset_news() {
        let (_, data) = Command::SetSoundMode(SoundMode::News).encode().unwrap();
        assert_eq!(data, vec![0xAA, 0x03, 6, 0x00]);
    }

    #[test]
    fn encode_set_balance() {
        let (uuid, data) = Command::SetBalance(50).encode().unwrap();
        assert_eq!(uuid, CHAR_AUDIO_PATH);
        assert_eq!(data, vec![0xAA, 0x04, 0x00, 0x01, 50]);
    }

    #[test]
    fn encode_set_power_on() {
        let (uuid, data) = Command::SetPower(true).encode().unwrap();
        assert_eq!(uuid, CHAR_AUDIO_PATH);
        assert_eq!(data, vec![0xAA, 0x04, 0x01, 0x01, 1]);
    }

    #[test]
    fn encode_set_power_off() {
        let (_, data) = Command::SetPower(false).encode().unwrap();
        assert_eq!(data, vec![0xAA, 0x04, 0x01, 0x01, 0]);
    }

    #[test]
    fn encode_set_source() {
        let (uuid, data) = Command::SetInput(Input::HdmiArc).encode().unwrap();
        assert_eq!(uuid, CHAR_SOURCE);
        assert_eq!(data, vec![0xAA, 0x07, 0, 0x00]); // Format B

        let (_, data) = Command::SetInput(Input::Bluetooth).encode().unwrap();
        assert_eq!(data, vec![0xAA, 0x07, 1, 0x00]);

        let (_, data) = Command::SetInput(Input::Aux).encode().unwrap();
        assert_eq!(data, vec![0xAA, 0x07, 2, 0x00]);

        let (_, data) = Command::SetInput(Input::Optical).encode().unwrap();
        assert_eq!(data, vec![0xAA, 0x07, 3, 0x00]);
    }

    #[test]
    fn encode_get_state() {
        let (uuid, data) = Command::GetState.encode().unwrap();
        assert_eq!(uuid, CHAR_DEVICE_INFO);
        assert_eq!(data, vec![0xAA, 0x01, 0x01, 0x00]);
    }

    #[test]
    fn encode_get_firmware_version() {
        let (uuid, data) = Command::GetFirmwareVersion.encode().unwrap();
        assert_eq!(uuid, CHAR_DEVICE_INFO);
        assert_eq!(data, vec![0xAA, 0x01, 0x01, 0x01]);
    }

    #[test]
    fn encode_play_pause() {
        let (uuid, data) = Command::SetPlayPause(1).encode().unwrap();
        assert_eq!(uuid, CHAR_PLAYER_CONTROL);
        assert_eq!(data, vec![0xAA, 0x05, 0x00, 0x01, 1]);
    }

    // --- Validation tests ---

    #[test]
    fn validate_volume_too_high() {
        assert!(Command::SetVolume(37).validate().is_err());
        assert!(Command::SetVolume(36).validate().is_ok());
    }

    #[test]
    fn validate_bass_too_high() {
        assert!(Command::SetBass(21).validate().is_err());
        assert!(Command::SetBass(20).validate().is_ok());
    }

    #[test]
    fn validate_treble_too_high() {
        assert!(Command::SetTreble(21).validate().is_err());
    }

    #[test]
    fn validate_center_volume_too_high() {
        assert!(Command::SetCenterVolume(31).validate().is_err());
    }

    #[test]
    fn validate_rear_volume_too_high() {
        assert!(Command::SetRearChannelVolume(31).validate().is_err());
    }

    #[test]
    fn validate_balance_too_high() {
        assert!(Command::SetBalance(101).validate().is_err());
        assert!(Command::SetBalance(100).validate().is_ok());
    }

    // --- Response decoding tests ---

    fn make_notification(code: u8, value: u8) -> Vec<u8> {
        // Typical notification: CC 05 AA <stuff> <code> <value>
        vec![0xCC, 0x05, 0xAA, 0x00, code, value]
    }

    #[test]
    fn decode_volume() {
        let data = make_notification(0x01, 18);
        assert_eq!(Response::decode(CHAR_UPSTREAM, &data), Response::Volume(18));
    }

    #[test]
    fn decode_center_volume() {
        let data = make_notification(0x02, 15);
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::CenterVolume(15)
        );
    }

    #[test]
    fn decode_treble() {
        let data = make_notification(0x03, 10);
        assert_eq!(Response::decode(CHAR_UPSTREAM, &data), Response::Treble(10));
    }

    #[test]
    fn decode_bass() {
        let data = make_notification(0x04, 12);
        assert_eq!(Response::decode(CHAR_UPSTREAM, &data), Response::Bass(12));
    }

    #[test]
    fn decode_mute_on() {
        let data = make_notification(0x05, 1);
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::MuteState(true)
        );
    }

    #[test]
    fn decode_mute_off() {
        let data = make_notification(0x05, 0);
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::MuteState(false)
        );
    }

    #[test]
    fn decode_quiet_mode() {
        let data = make_notification(0x06, 1);
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::QuietMode(true)
        );
    }

    #[test]
    fn decode_balance() {
        let data = make_notification(0x07, 50);
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::Balance(50)
        );
    }

    #[test]
    fn decode_source_hdmi() {
        let data = make_notification(0x09, 0);
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::CurrentInput(Input::HdmiArc)
        );
    }

    #[test]
    fn decode_source_optical() {
        let data = make_notification(0x09, 3);
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::CurrentInput(Input::Optical)
        );
    }

    #[test]
    fn decode_power_on_is_zero() {
        // Power is INVERTED: 0 = ON
        let data = make_notification(0x0A, 0);
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::Power(true)
        );
    }

    #[test]
    fn decode_power_off_is_one() {
        // Power is INVERTED: 1 = OFF
        let data = make_notification(0x0A, 1);
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::Power(false)
        );
    }

    #[test]
    fn decode_preset_movies() {
        let data = make_notification(0x0B, 0);
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::CurrentSoundMode(SoundMode::Movies)
        );
    }

    #[test]
    fn decode_preset_music() {
        let data = make_notification(0x0B, 1);
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::CurrentSoundMode(SoundMode::Music)
        );
    }

    #[test]
    fn decode_preset_tv() {
        let data = make_notification(0x0B, 2);
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::CurrentSoundMode(SoundMode::Tv)
        );
    }

    #[test]
    fn decode_preset_news() {
        let data = make_notification(0x0B, 3);
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::CurrentSoundMode(SoundMode::News)
        );
    }

    #[test]
    fn decode_subwoofer_connected() {
        let data = make_notification(0x0E, 1);
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::SubwooferConnected(true)
        );
    }

    #[test]
    fn decode_rear_volume() {
        let data = make_notification(0x0F, 20);
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::RearVolume(20)
        );
    }

    #[test]
    fn decode_firmware_version() {
        // CC 06 AA 01 03 01 01 47 = MCU version 1.71
        let data = vec![0xCC, 0x06, 0xAA, 0x01, 0x03, 0x01, 0x01, 0x47];
        assert_eq!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::FirmwareVersion {
                fw_type: 0x01,
                major: 0x01,
                minor: 0x47,
            }
        );
    }

    #[test]
    fn decode_short_data_returns_unknown() {
        let data = vec![0xCC, 0x05, 0xAA];
        assert!(matches!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::Unknown { .. }
        ));
    }

    #[test]
    fn decode_out_of_range_code_returns_unknown() {
        let data = make_notification(0xFF, 0x00);
        assert!(matches!(
            Response::decode(CHAR_UPSTREAM, &data),
            Response::Unknown { .. }
        ));
    }

    // --- SoundMode write/read roundtrip ---

    #[test]
    fn sound_mode_write_values_match_homebridge() {
        assert_eq!(SoundMode::Tv.to_write_byte(), 5);
        assert_eq!(SoundMode::News.to_write_byte(), 6);
        assert_eq!(SoundMode::Movies.to_write_byte(), 7);
        assert_eq!(SoundMode::Music.to_write_byte(), 8);
    }

    #[test]
    fn sound_mode_read_values_match_homebridge() {
        assert_eq!(SoundMode::from_read_byte(0).unwrap(), SoundMode::Movies);
        assert_eq!(SoundMode::from_read_byte(1).unwrap(), SoundMode::Music);
        assert_eq!(SoundMode::from_read_byte(2).unwrap(), SoundMode::Tv);
        assert_eq!(SoundMode::from_read_byte(3).unwrap(), SoundMode::News);
    }

    // --- Input byte values match homebridge ---

    #[test]
    fn input_values_match_homebridge() {
        assert_eq!(Input::HdmiArc.to_byte(), 0);
        assert_eq!(Input::Bluetooth.to_byte(), 1);
        assert_eq!(Input::Aux.to_byte(), 2);
        assert_eq!(Input::Optical.to_byte(), 3);
    }

    // --- UUID tests ---

    #[test]
    fn service_uuid_encodes_excelpoint() {
        let bytes = SERVICE_STEALTHTECH.as_bytes();
        let ascii: Vec<u8> = "excelpoint.com".bytes().collect();
        assert_eq!(&bytes[..14], &ascii[..]);
    }

    #[test]
    fn characteristic_uuids_share_base() {
        let service_bytes = SERVICE_STEALTHTECH.as_bytes();
        let upstream_bytes = CHAR_UPSTREAM.as_bytes();
        // First 14 bytes should match
        assert_eq!(&service_bytes[..14], &upstream_bytes[..14]);
        // Last 2 bytes differ
        assert_eq!(upstream_bytes[14], 0x00);
        assert_eq!(upstream_bytes[15], 0x01);
    }
}
