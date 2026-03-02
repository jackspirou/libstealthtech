//! GATT service/characteristic UUIDs and protocol constants for StealthTech.
//!
//! # Protocol Discovery
//!
//! The StealthTech BLE protocol was reverse-engineered from:
//! - **Firmware binary analysis** (`mcu.bin` string extraction from Lovesac_Firmware_V1.71)
//! - **[homebridge-lovesac-stealthtech](https://github.com/ohmantics/homebridge-lovesac-stealthtech)**
//!   (MIT, Alex Rosenberg) — confirmed UUIDs, packet formats, and value ranges
//!
//! # Hardware Architecture
//!
//! The center channel uses a **Qualcomm QCC3008** Bluetooth 5.0 SoC that
//! communicates with an ARM Cortex-M MCU via UART AT commands. BLE GATT
//! writes arrive at the QCC3008 and are forwarded to the MCU via `AT+GATTD`.
//!
//! # Custom Service UUID
//!
//! The StealthTech GATT service UUID encodes **"excelpoint.com"** in ASCII:
//! ```text
//! 65 78 63 65 6c 70 6f 69 6e 74 2e 63 6f 6d 00 00
//!  e  x  c  e  l  p  o  i  n  t  .  c  o  m
//! ```
//! [Excelpoint Technology](https://www.excelpoint.com) is a Singapore-based
//! electronics distributor that likely designed the BLE firmware for Harman.
//!
//! # Characteristic Map
//!
//! All characteristics share the same base UUID with the last 2 bytes varying:
//!
//! | Char | UUID Suffix | Purpose | Commands |
//! |------|-------------|---------|----------|
//! | UpStream | 0001 | Notifications FROM device | Subscribe for state changes |
//! | DeviceInfo | 0002 | State/version requests | GetState, GetFirmwareVersion |
//! | EqControl | 0003 | Audio EQ + presets | Volume, Bass, Treble, Center, Rear, Mute, Quiet, Preset |
//! | AudioPath | 0004 | Audio routing | Balance, Power |
//! | PlayerControl | 0005 | BT media control | Play/Pause, Skip |
//! | SystemLayout | 0006 | Physical config | Configuration shape |
//! | Source | 0007 | Input selection | HDMI, Optical, BT, AUX |
//! | Covering | 0008 | Acoustic tuning | Fabric type |
//! | UserSetting | 0009 | User preferences | (details TBD) |
//! | OTA | 000a | Firmware update | (DFU protocol) |

use uuid::Uuid;

// ============================================================================
// Device discovery constants
// ============================================================================

/// Known BLE advertisement names for StealthTech center channels.
/// Used as a fallback for device discovery during scanning.
/// Primary discovery should use [`SERVICE_STEALTHTECH`] UUID matching.
pub const STEALTHTECH_DEVICE_NAMES: &[&str] = &[
    "stealthtech",
    "stealth tech",
    "lovesac",
    "sound + charge",
    "sound+charge",
    // Harman Kardon internal names (may appear on some firmware versions)
    "hk_lovesac",
    "ee4034",
];

/// Maximum volume level (0-36 scale, NOT 0-100).
/// Confirmed by homebridge-lovesac-stealthtech.
pub const MAX_VOLUME: u8 = 36;

/// Maximum bass level.
pub const MAX_BASS: u8 = 20;

/// Maximum treble level.
pub const MAX_TREBLE: u8 = 20;

/// Maximum center channel volume.
pub const MAX_CENTER_VOLUME: u8 = 30;

/// Maximum rear channel volume.
pub const MAX_REAR_VOLUME: u8 = 30;

/// Maximum balance value (50 = center).
pub const MAX_BALANCE: u8 = 100;

// ============================================================================
// Helper: build a StealthTech UUID from the last 2 bytes
// ============================================================================

/// Build a full 128-bit StealthTech UUID from a 16-bit suffix.
///
/// The base is `65786365-6c70-6f69-6e74-2e636f6d` ("excelpoint.com")
/// and the suffix occupies the last 2 bytes.
const fn stealthtech_uuid(suffix: u16) -> Uuid {
    Uuid::from_bytes([
        0x65,
        0x78,
        0x63,
        0x65, // "exce"
        0x6c,
        0x70, // "lp"
        0x6f,
        0x69, // "oi"
        0x6e,
        0x74, // "nt"
        0x2e,
        0x63,
        0x6f,
        0x6d, // ".com"
        (suffix >> 8) as u8,
        (suffix & 0xFF) as u8,
    ])
}

// ============================================================================
// Standard BLE Service UUIDs
// ============================================================================

/// Generic Access Service (0x1800).
pub const SERVICE_GENERIC_ACCESS: Uuid = Uuid::from_bytes([
    0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0x80, 0x5f, 0x9b, 0x34, 0xfb,
]);

/// Generic Attribute Service (0x1801).
pub const SERVICE_GENERIC_ATTRIBUTE: Uuid = Uuid::from_bytes([
    0x00, 0x00, 0x18, 0x01, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0x80, 0x5f, 0x9b, 0x34, 0xfb,
]);

/// Device Information Service (0x180A).
pub const SERVICE_DEVICE_INFORMATION: Uuid = Uuid::from_bytes([
    0x00, 0x00, 0x18, 0x0A, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0x80, 0x5f, 0x9b, 0x34, 0xfb,
]);

// ============================================================================
// StealthTech Custom Service UUID
// ============================================================================

/// Primary StealthTech GATT service.
///
/// UUID: `65786365-6c70-6f69-6e74-2e636f6d0000`
///
/// The UUID encodes "excelpoint.com" in ASCII — Excelpoint Technology is a
/// Singapore-based electronics distributor that designed the BLE firmware.
/// All StealthTech characteristics live under this single service.
pub const SERVICE_STEALTHTECH: Uuid = stealthtech_uuid(0x0000);

// ============================================================================
// StealthTech Characteristic UUIDs
// ============================================================================

/// **UpStream** — notification characteristic for device → host status updates.
///
/// Subscribe to this characteristic to receive state change notifications.
/// Notification format: `CC 05/06 AA ... <response_code> <value>`
/// The last 2 bytes are always the response code and value.
///
/// Version responses have the format: `CC 05/06 AA 01 03 <type> <major> <minor>`
/// where type: 0x01=MCU, 0x02=DSP, 0x03=EQ.
pub const CHAR_UPSTREAM: Uuid = stealthtech_uuid(0x0001);

/// **DeviceInfo** — request device state dump or firmware version.
///
/// Write `AA 01 01 00` (Format B) to request full state dump.
/// Write `AA 01 01 01` to request firmware version info.
/// Responses arrive on [`CHAR_UPSTREAM`].
pub const CHAR_DEVICE_INFO: Uuid = stealthtech_uuid(0x0002);

/// **EqControl** — audio EQ, volume, mute, quiet mode, and preset control.
///
/// This is the most-used characteristic. Commands use Format A or B:
/// - Volume: Format A, cmd=0x03, sub=0x02, value 0-36
/// - Bass: Format A, cmd=0x03, sub=0x01, value 0-20
/// - Treble: Format A, cmd=0x03, sub=0x00, value 0-20
/// - Center vol: Format A, cmd=0x03, sub=0x03, value 0-30
/// - Rear vol: Format A, cmd=0x03, sub=0x0A, value 0-30
/// - Mute: Format A, cmd=0x03, sub=0x09, value 0/1
/// - Quiet mode: Format A, cmd=0x03, sub=0x04, value 0/1
/// - Preset: Format B, cmd=0x03, value 5-9
pub const CHAR_EQ_CONTROL: Uuid = stealthtech_uuid(0x0003);

/// **AudioPath** — balance and power control.
///
/// - Balance: Format A, cmd=0x04, sub=0x00, value 0-100 (50=center)
/// - Power: Format A, cmd=0x04, sub=0x01, value 0/1
pub const CHAR_AUDIO_PATH: Uuid = stealthtech_uuid(0x0004);

/// **PlayerControl** — Bluetooth media playback control.
///
/// - Play/Pause: Format A, cmd=0x05, sub=0x00, value
/// - Skip fwd/back: Format A, cmd=0x05, sub=0x01, value
pub const CHAR_PLAYER_CONTROL: Uuid = stealthtech_uuid(0x0005);

/// **SystemLayout** — physical configuration shape for surround calibration.
///
/// MCU variable: `AppDate.SystemLayoutVal`.
pub const CHAR_SYSTEM_LAYOUT: Uuid = stealthtech_uuid(0x0006);

/// **Source** — audio input source selection.
///
/// Format B, cmd=0x07, value: HDMI=0, Bluetooth=1, AUX=2, Optical=3.
/// MCU variable: `gSys.Source_State`.
pub const CHAR_SOURCE: Uuid = stealthtech_uuid(0x0007);

/// **Covering** — fabric type for acoustic tuning.
///
/// MCU variable: `AppDate.CovingVal` (single byte ID).
pub const CHAR_COVERING: Uuid = stealthtech_uuid(0x0008);

/// **UserSetting** — user preferences.
pub const CHAR_USER_SETTING: Uuid = stealthtech_uuid(0x0009);

/// **OTA** — over-the-air firmware update.
pub const CHAR_OTA: Uuid = stealthtech_uuid(0x000A);

// ============================================================================
// Response codes (from UpStream notifications)
// ============================================================================

/// Response codes in the second-to-last byte of UpStream notifications.
pub mod response_code {
    pub const VOLUME: u8 = 0x01;
    pub const CENTER_VOLUME: u8 = 0x02;
    pub const TREBLE: u8 = 0x03;
    pub const BASS: u8 = 0x04;
    pub const MUTE: u8 = 0x05;
    pub const QUIET_MODE: u8 = 0x06;
    pub const BALANCE: u8 = 0x07;
    pub const LAYOUT: u8 = 0x08;
    pub const SOURCE: u8 = 0x09;
    pub const POWER: u8 = 0x0A;
    pub const PRESET: u8 = 0x0B;
    pub const COVERING: u8 = 0x0C;
    pub const ARM_TYPE: u8 = 0x0D;
    pub const SUBWOOFER: u8 = 0x0E;
    pub const REAR_VOLUME: u8 = 0x0F;

    pub const MIN: u8 = VOLUME;
    pub const MAX: u8 = REAR_VOLUME;
}

// ============================================================================
// Known firmware version strings
// ============================================================================

/// A firmware component version (major, minor).
///
/// The device reports three components: MCU (fw_type=1), DSP (fw_type=2),
/// and EQ (fw_type=3). Each has an independent major.minor version number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FirmwareComponentVersion {
    pub major: u8,
    pub minor: u8,
}

impl FirmwareComponentVersion {
    pub const fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }

    /// Returns true if this version is at least as new as `other`.
    pub fn is_at_least(&self, other: &Self) -> bool {
        (self.major, self.minor) >= (other.major, other.minor)
    }
}

impl std::fmt::Display for FirmwareComponentVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}.{}", self.major, self.minor)
    }
}

/// Latest known firmware component versions (from V-1.71 package).
///
/// MCU reports v1.71 via BLE (the "package version"), DSP reports v1.68,
/// EQ reports v1.23. These values come from the BLE `GetFirmwareVersion`
/// response, not from `LS_config.txt` (which shows different MCU numbering).
pub const LATEST_MCU_VERSION: FirmwareComponentVersion = FirmwareComponentVersion::new(1, 71);
pub const LATEST_DSP_VERSION: FirmwareComponentVersion = FirmwareComponentVersion::new(1, 68);
pub const LATEST_EQ_VERSION: FirmwareComponentVersion = FirmwareComponentVersion::new(1, 23);

// ============================================================================
// Known model numbers
// ============================================================================

/// StealthTech hardware model numbers from FCC filings and setup guides.
pub const MODEL_CENTER_CHANNEL: &str = "EE4034";
pub const MODEL_CENTER_CHANNEL_R2: &str = "GA4364";
pub const MODEL_SUBWOOFER: &str = "EE0362";
pub const MODEL_SOUND_CHARGE_SIDE_L: &str = "GE2913";
pub const MODEL_SOUND_CHARGE_SIDE_R: &str = "GE0177";
pub const MODEL_SATELLITE_SIDE: &str = "GE7065";
pub const MODEL_REMOTE: &str = "EE3531";
pub const MODEL_CHARGING_PAD: &str = "QST008A";
