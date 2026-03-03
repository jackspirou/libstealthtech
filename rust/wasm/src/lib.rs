//! WebAssembly bindings for the StealthTech BLE protocol.
//!
//! This crate wraps [`libstealthtech_protocol`] types for use from
//! JavaScript/TypeScript via `wasm-bindgen`. It provides:
//!
//! - Protocol constant exports (volume/bass/treble limits)
//! - Command encoding (JSON in, UUID + bytes out)
//! - Response decoding (UUID + bytes in, JSON out)
//! - Device state tracking via [`WasmDeviceState`]
//! - Characteristic UUID accessors

use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

use libstealthtech_protocol::characteristics::{
    CHAR_AUDIO_PATH, CHAR_COVERING, CHAR_DEVICE_INFO, CHAR_EQ_CONTROL, CHAR_OTA,
    CHAR_PLAYER_CONTROL, CHAR_SOURCE, CHAR_SYSTEM_LAYOUT, CHAR_UPSTREAM, CHAR_USER_SETTING,
    LATEST_DSP_VERSION, LATEST_EQ_VERSION, LATEST_MCU_VERSION, MAX_BALANCE, MAX_BASS,
    MAX_CENTER_VOLUME, MAX_REAR_VOLUME, MAX_TREBLE, MAX_VOLUME, SERVICE_STEALTHTECH,
};
use libstealthtech_protocol::commands::{Command, ConfigShape, Input, Response, SoundMode};
use libstealthtech_protocol::DeviceState;

// ============================================================================
// Protocol constants
// ============================================================================

/// Maximum volume level (0-36).
#[wasm_bindgen]
pub fn max_volume() -> u8 {
    MAX_VOLUME
}

/// Maximum bass level (0-20).
#[wasm_bindgen]
pub fn max_bass() -> u8 {
    MAX_BASS
}

/// Maximum treble level (0-20).
#[wasm_bindgen]
pub fn max_treble() -> u8 {
    MAX_TREBLE
}

/// Maximum center channel volume (0-30).
#[wasm_bindgen]
pub fn max_center_volume() -> u8 {
    MAX_CENTER_VOLUME
}

/// Maximum rear channel volume (0-30).
#[wasm_bindgen]
pub fn max_rear_volume() -> u8 {
    MAX_REAR_VOLUME
}

/// Maximum balance value (0-100, 50=center).
#[wasm_bindgen]
pub fn max_balance() -> u8 {
    MAX_BALANCE
}

// ============================================================================
// Characteristic UUID exports
// ============================================================================

/// Get the StealthTech service UUID as a string.
#[wasm_bindgen]
pub fn service_uuid() -> String {
    SERVICE_STEALTHTECH.to_string()
}

/// Get the UpStream (notification) characteristic UUID as a string.
#[wasm_bindgen]
pub fn upstream_char_uuid() -> String {
    CHAR_UPSTREAM.to_string()
}

/// Get all characteristic UUIDs as a JSON map.
///
/// Returns a JSON object mapping human-readable names to UUID strings:
/// ```json
/// {
///   "service": "65786365-...",
///   "upstream": "65786365-...",
///   "device_info": "65786365-...",
///   ...
/// }
/// ```
#[wasm_bindgen]
pub fn characteristic_uuids() -> String {
    let map = json!({
        "service": SERVICE_STEALTHTECH.to_string(),
        "upstream": CHAR_UPSTREAM.to_string(),
        "device_info": CHAR_DEVICE_INFO.to_string(),
        "eq_control": CHAR_EQ_CONTROL.to_string(),
        "audio_path": CHAR_AUDIO_PATH.to_string(),
        "player_control": CHAR_PLAYER_CONTROL.to_string(),
        "system_layout": CHAR_SYSTEM_LAYOUT.to_string(),
        "source": CHAR_SOURCE.to_string(),
        "covering": CHAR_COVERING.to_string(),
        "user_setting": CHAR_USER_SETTING.to_string(),
        "ota": CHAR_OTA.to_string(),
    });
    map.to_string()
}

// ============================================================================
// Command encoding
// ============================================================================

/// Encode a command to bytes for BLE transmission.
///
/// Takes a JSON command description and returns the target characteristic UUID
/// and byte payload as JSON.
///
/// # Input format (JSON string)
///
/// ```text
/// {"SetVolume": 18}
/// {"SetBass": 10}
/// {"SetTreble": 15}
/// {"SetCenterVolume": 20}
/// {"SetRearChannelVolume": 25}
/// {"SetMute": true}
/// {"SetQuietCouch": false}
/// {"SetSoundMode": "Movies"}
/// {"SetBalance": 50}
/// {"SetPower": true}
/// {"SetInput": "HdmiArc"}
/// {"SetFabric": 2}
/// {"SetConfigShape": "LShape"}
/// {"SetArmType": 1}
/// {"SetPlayPause": 1}
/// {"SetSkip": 0}
/// "GetState"
/// "GetFirmwareVersion"
/// ```
///
/// # Returns
///
/// JSON string: `{"uuid": "65786365-...", "data": [170, 3, 2, 1, 18]}`
#[wasm_bindgen]
pub fn encode_command(cmd_json: &str) -> Result<String, JsError> {
    let cmd = parse_command(cmd_json)?;
    let (uuid, data) = cmd.encode().map_err(|e| JsError::new(&e.to_string()))?;
    let result = json!({
        "uuid": uuid.to_string(),
        "data": data,
    });
    Ok(result.to_string())
}

/// Parse a JSON string into a `Command` variant.
fn parse_command(cmd_json: &str) -> Result<Command, JsError> {
    let value: Value =
        serde_json::from_str(cmd_json).map_err(|e| JsError::new(&format!("invalid JSON: {e}")))?;

    match &value {
        // Unit variants: "GetState", "GetFirmwareVersion"
        Value::String(s) => match s.as_str() {
            "GetState" => Ok(Command::GetState),
            "GetFirmwareVersion" => Ok(Command::GetFirmwareVersion),
            other => Err(JsError::new(&format!("unknown command string: {other}"))),
        },

        // Object variants: {"SetVolume": 18}, {"SetInput": "HdmiArc"}, etc.
        Value::Object(map) => {
            if map.len() != 1 {
                return Err(JsError::new("command object must have exactly one key"));
            }
            let (key, val) = map.iter().next().unwrap();
            match key.as_str() {
                "SetVolume" => {
                    let v = val_to_u8(val, "SetVolume")?;
                    Ok(Command::SetVolume(v))
                }
                "SetBass" => {
                    let v = val_to_u8(val, "SetBass")?;
                    Ok(Command::SetBass(v))
                }
                "SetTreble" => {
                    let v = val_to_u8(val, "SetTreble")?;
                    Ok(Command::SetTreble(v))
                }
                "SetCenterVolume" => {
                    let v = val_to_u8(val, "SetCenterVolume")?;
                    Ok(Command::SetCenterVolume(v))
                }
                "SetRearChannelVolume" => {
                    let v = val_to_u8(val, "SetRearChannelVolume")?;
                    Ok(Command::SetRearChannelVolume(v))
                }
                "SetMute" => {
                    let b = val_to_bool(val, "SetMute")?;
                    Ok(Command::SetMute(b))
                }
                "SetQuietCouch" => {
                    let b = val_to_bool(val, "SetQuietCouch")?;
                    Ok(Command::SetQuietCouch(b))
                }
                "SetSoundMode" => {
                    let mode = val_to_sound_mode(val)?;
                    Ok(Command::SetSoundMode(mode))
                }
                "SetBalance" => {
                    let v = val_to_u8(val, "SetBalance")?;
                    Ok(Command::SetBalance(v))
                }
                "SetPower" => {
                    let b = val_to_bool(val, "SetPower")?;
                    Ok(Command::SetPower(b))
                }
                "SetInput" => {
                    let input = val_to_input(val)?;
                    Ok(Command::SetInput(input))
                }
                "SetFabric" => {
                    let v = val_to_u8(val, "SetFabric")?;
                    Ok(Command::SetFabric(v))
                }
                "SetConfigShape" => {
                    let shape = val_to_config_shape(val)?;
                    Ok(Command::SetConfigShape(shape))
                }
                "SetArmType" => {
                    let v = val_to_u8(val, "SetArmType")?;
                    Ok(Command::SetArmType(v))
                }
                "SetPlayPause" => {
                    let v = val_to_u8(val, "SetPlayPause")?;
                    Ok(Command::SetPlayPause(v))
                }
                "SetSkip" => {
                    let v = val_to_u8(val, "SetSkip")?;
                    Ok(Command::SetSkip(v))
                }
                other => Err(JsError::new(&format!("unknown command key: {other}"))),
            }
        }

        _ => Err(JsError::new("command must be a JSON string or object")),
    }
}

/// Extract a `u8` from a JSON value.
fn val_to_u8(val: &Value, field: &str) -> Result<u8, JsError> {
    val.as_u64()
        .and_then(|n| u8::try_from(n).ok())
        .ok_or_else(|| JsError::new(&format!("{field} requires a u8 integer value")))
}

/// Extract a `bool` from a JSON value.
fn val_to_bool(val: &Value, field: &str) -> Result<bool, JsError> {
    val.as_bool()
        .ok_or_else(|| JsError::new(&format!("{field} requires a boolean value")))
}

/// Parse a `SoundMode` from a JSON string value.
fn val_to_sound_mode(val: &Value) -> Result<SoundMode, JsError> {
    let s = val
        .as_str()
        .ok_or_else(|| JsError::new("SetSoundMode requires a string value"))?;
    match s {
        "Movies" => Ok(SoundMode::Movies),
        "Music" => Ok(SoundMode::Music),
        "Tv" => Ok(SoundMode::Tv),
        "News" => Ok(SoundMode::News),
        "Manual" => Ok(SoundMode::Manual),
        other => Err(JsError::new(&format!(
            "unknown sound mode: {other} (expected Movies, Music, Tv, News, or Manual)"
        ))),
    }
}

/// Parse an `Input` from a JSON string value.
fn val_to_input(val: &Value) -> Result<Input, JsError> {
    let s = val
        .as_str()
        .ok_or_else(|| JsError::new("SetInput requires a string value"))?;
    match s {
        "HdmiArc" => Ok(Input::HdmiArc),
        "Bluetooth" => Ok(Input::Bluetooth),
        "Aux" => Ok(Input::Aux),
        "Optical" => Ok(Input::Optical),
        other => Err(JsError::new(&format!(
            "unknown input: {other} (expected HdmiArc, Bluetooth, Aux, or Optical)"
        ))),
    }
}

/// Parse a `ConfigShape` from a JSON string value.
fn val_to_config_shape(val: &Value) -> Result<ConfigShape, JsError> {
    let s = val
        .as_str()
        .ok_or_else(|| JsError::new("SetConfigShape requires a string value"))?;
    match s {
        "Straight" => Ok(ConfigShape::Straight),
        "LShape" => Ok(ConfigShape::LShape),
        "UShape" => Ok(ConfigShape::UShape),
        "Pit" => Ok(ConfigShape::Pit),
        other => Err(JsError::new(&format!(
            "unknown config shape: {other} (expected Straight, LShape, UShape, or Pit)"
        ))),
    }
}

// ============================================================================
// Response decoding
// ============================================================================

/// Decode a BLE notification into a typed response.
///
/// Takes the characteristic UUID string and raw notification bytes, and returns
/// a JSON description of the decoded response.
///
/// # Returns
///
/// JSON string like:
/// ```text
/// {"Volume": 18}
/// {"CurrentInput": "HdmiArc"}
/// {"Power": true}
/// {"FirmwareVersion": {"fw_type": 1, "major": 1, "minor": 71}}
/// {"Unknown": {"uuid": "65786365-...", "data": [204, 5, 170]}}
/// ```
#[wasm_bindgen]
pub fn decode_response(uuid_str: &str, data: &[u8]) -> Result<String, JsError> {
    let uuid =
        uuid::Uuid::parse_str(uuid_str).map_err(|e| JsError::new(&format!("invalid UUID: {e}")))?;

    let response = Response::decode(uuid, data);
    let json = serde_json::to_string(&response)
        .map_err(|e| JsError::new(&format!("serialize error: {e}")))?;
    Ok(json)
}

// ============================================================================
// DeviceState wrapper
// ============================================================================

/// Tracked device state, updated by applying decoded responses.
///
/// Create a new instance, then call `apply_response()` with JSON from
/// `decode_response()` to keep it in sync with the physical device.
#[wasm_bindgen]
pub struct WasmDeviceState {
    inner: DeviceState,
}

impl Default for WasmDeviceState {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmDeviceState {
    /// Create a new empty device state.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: DeviceState::default(),
        }
    }

    /// Update state from a decoded response JSON string.
    ///
    /// Accepts the same JSON format returned by `decode_response()`.
    /// Returns an error if the JSON does not match a valid `Response` variant.
    pub fn apply_response(&mut self, response_json: &str) -> Result<(), JsError> {
        let response: Response = serde_json::from_str(response_json)
            .map_err(|e| JsError::new(&format!("invalid response JSON: {e}")))?;
        self.inner.apply_response(&response);
        Ok(())
    }

    /// Get the full device state as a JSON string.
    pub fn to_json(&self) -> Result<String, JsError> {
        serde_json::to_string(&self.inner)
            .map_err(|e| JsError::new(&format!("serialization error: {e}")))
    }

    /// Get the current volume level, if known.
    pub fn volume(&self) -> Option<u8> {
        self.inner.volume
    }

    /// Get the current bass level, if known.
    pub fn bass(&self) -> Option<u8> {
        self.inner.bass
    }

    /// Get the current treble level, if known.
    pub fn treble(&self) -> Option<u8> {
        self.inner.treble
    }

    /// Get the current center channel volume, if known.
    pub fn center_volume(&self) -> Option<u8> {
        self.inner.center_volume
    }

    /// Get the current rear channel volume, if known.
    pub fn rear_channel_volume(&self) -> Option<u8> {
        self.inner.rear_channel_volume
    }

    /// Get the current balance value, if known.
    pub fn balance(&self) -> Option<u8> {
        self.inner.balance
    }

    /// Get whether the device is muted, if known.
    pub fn mute(&self) -> Option<bool> {
        self.inner.mute
    }

    /// Get whether the device is powered on, if known.
    pub fn power(&self) -> Option<bool> {
        self.inner.power
    }

    /// Get whether quiet couch mode is active, if known.
    pub fn quiet_couch(&self) -> Option<bool> {
        self.inner.quiet_couch
    }

    /// Get whether the subwoofer is connected, if known.
    pub fn subwoofer_connected(&self) -> Option<bool> {
        self.inner.subwoofer_connected
    }

    /// Get the fabric covering type ID, if known.
    pub fn fabric(&self) -> Option<u8> {
        self.inner.fabric
    }

    /// Get the arm type ID, if known.
    pub fn arm_type(&self) -> Option<u8> {
        self.inner.arm_type
    }

    /// Get the current couch configuration shape as a display string, if known.
    pub fn config_shape(&self) -> Option<String> {
        self.inner.config_shape.map(|s| format!("{s}"))
    }

    /// Get the current input source as a string, if known.
    pub fn input(&self) -> Option<String> {
        self.inner.input.map(|i| match i {
            Input::HdmiArc => "HdmiArc".to_string(),
            Input::Bluetooth => "Bluetooth".to_string(),
            Input::Aux => "Aux".to_string(),
            Input::Optical => "Optical".to_string(),
        })
    }

    /// Get the current sound mode as a string, if known.
    pub fn sound_mode(&self) -> Option<String> {
        self.inner.sound_mode.map(|m| match m {
            SoundMode::Movies => "Movies".to_string(),
            SoundMode::Music => "Music".to_string(),
            SoundMode::Tv => "Tv".to_string(),
            SoundMode::News => "News".to_string(),
            SoundMode::Manual => "Manual".to_string(),
        })
    }

    /// Get the firmware version string, if known.
    pub fn firmware_version(&self) -> Option<String> {
        self.inner.firmware_version.clone()
    }

    /// Get firmware status as a JSON string.
    ///
    /// Returns a JSON object with per-component versions, latest known
    /// versions, and whether an update is available:
    /// ```json
    /// {
    ///   "mcu": {"current": "v1.71", "latest": "v1.71", "up_to_date": true},
    ///   "dsp": {"current": "v1.68", "latest": "v1.68", "up_to_date": true},
    ///   "eq": {"current": "v1.23", "latest": "v1.23", "up_to_date": true},
    ///   "update_available": false
    /// }
    /// ```
    pub fn firmware_status(&self) -> Option<String> {
        if self.inner.mcu_version.is_none()
            && self.inner.dsp_version.is_none()
            && self.inner.eq_version.is_none()
        {
            return None;
        }

        let component =
            |current: Option<
                libstealthtech_protocol::characteristics::FirmwareComponentVersion,
            >,
             latest: &libstealthtech_protocol::characteristics::FirmwareComponentVersion|
             -> Value {
                match current {
                    Some(v) => json!({
                        "current": format!("{v}"),
                        "latest": format!("{latest}"),
                        "up_to_date": v.is_at_least(latest),
                    }),
                    None => json!({
                        "current": null,
                        "latest": format!("{latest}"),
                        "up_to_date": null,
                    }),
                }
            };

        let status = json!({
            "mcu": component(self.inner.mcu_version, &LATEST_MCU_VERSION),
            "dsp": component(self.inner.dsp_version, &LATEST_DSP_VERSION),
            "eq": component(self.inner.eq_version, &LATEST_EQ_VERSION),
            "update_available": self.inner.firmware_update_available().unwrap_or(false),
        });

        Some(status.to_string())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Constant exports ---

    #[test]
    fn constants_match_protocol() {
        assert_eq!(max_volume(), 36);
        assert_eq!(max_bass(), 20);
        assert_eq!(max_treble(), 20);
        assert_eq!(max_center_volume(), 30);
        assert_eq!(max_rear_volume(), 30);
        assert_eq!(max_balance(), 100);
    }

    // --- UUID exports ---

    #[test]
    fn service_uuid_is_valid() {
        let uuid_str = service_uuid();
        assert!(uuid::Uuid::parse_str(&uuid_str).is_ok());
        assert!(uuid_str.contains("65786365"));
    }

    #[test]
    fn upstream_uuid_is_valid() {
        let uuid_str = upstream_char_uuid();
        assert!(uuid::Uuid::parse_str(&uuid_str).is_ok());
    }

    #[test]
    fn characteristic_uuids_returns_valid_json() {
        let json_str = characteristic_uuids();
        let value: Value = serde_json::from_str(&json_str).unwrap();
        let obj = value.as_object().unwrap();
        assert!(obj.contains_key("service"));
        assert!(obj.contains_key("upstream"));
        assert!(obj.contains_key("device_info"));
        assert!(obj.contains_key("eq_control"));
        assert!(obj.contains_key("audio_path"));
        assert!(obj.contains_key("player_control"));
        assert!(obj.contains_key("system_layout"));
        assert!(obj.contains_key("source"));
        assert!(obj.contains_key("covering"));
        assert!(obj.contains_key("user_setting"));
        assert!(obj.contains_key("ota"));
    }

    // --- Command encoding ---

    #[test]
    fn encode_set_volume_command() {
        let result = encode_command(r#"{"SetVolume": 18}"#).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.get("uuid").is_some());
        let data = parsed["data"].as_array().unwrap();
        assert_eq!(data[0], 0xAA);
        assert_eq!(data[4], 18);
    }

    #[test]
    fn encode_set_input_command() {
        let result = encode_command(r#"{"SetInput": "HdmiArc"}"#).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let data = parsed["data"].as_array().unwrap();
        assert_eq!(data[0], 0xAA);
        assert_eq!(data[1], 0x07); // Source cmd_id
        assert_eq!(data[2], 0); // HdmiArc = 0
    }

    #[test]
    fn encode_set_sound_mode_command() {
        let result = encode_command(r#"{"SetSoundMode": "Movies"}"#).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let data = parsed["data"].as_array().unwrap();
        assert_eq!(data[0], 0xAA);
        assert_eq!(data[2], 7); // Movies write byte = 7
    }

    #[test]
    fn encode_get_state_command() {
        let result = encode_command(r#""GetState""#).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let data = parsed["data"].as_array().unwrap();
        assert_eq!(data[0], 0xAA);
        assert_eq!(data[1], 0x01);
        assert_eq!(data[2], 0x01);
        assert_eq!(data[3], 0x00);
    }

    #[test]
    fn encode_get_firmware_version_command() {
        let result = encode_command(r#""GetFirmwareVersion""#).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let data = parsed["data"].as_array().unwrap();
        assert_eq!(data[0], 0xAA);
        assert_eq!(data[1], 0x01);
        assert_eq!(data[2], 0x01);
        assert_eq!(data[3], 0x01);
    }

    #[test]
    fn encode_set_mute_command() {
        let result = encode_command(r#"{"SetMute": true}"#).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let data = parsed["data"].as_array().unwrap();
        assert_eq!(data[4], 1);
    }

    #[test]
    fn encode_set_power_command() {
        let result = encode_command(r#"{"SetPower": false}"#).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let data = parsed["data"].as_array().unwrap();
        assert_eq!(data[4], 0);
    }

    #[test]
    fn encode_set_config_shape_command() {
        let result = encode_command(r#"{"SetConfigShape": "UShape"}"#).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let data = parsed["data"].as_array().unwrap();
        assert_eq!(data[2], 2); // UShape = 2
    }

    // Error-path tests require JsError which is only available on wasm32 targets.
    #[test]
    #[cfg(target_arch = "wasm32")]
    fn encode_invalid_json_returns_error() {
        assert!(encode_command("not valid json").is_err());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn encode_unknown_command_returns_error() {
        assert!(encode_command(r#""UnknownCommand""#).is_err());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn encode_volume_out_of_range_returns_error() {
        assert!(encode_command(r#"{"SetVolume": 255}"#).is_err());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn encode_wrong_value_type_returns_error() {
        assert!(encode_command(r#"{"SetVolume": "loud"}"#).is_err());
    }

    // --- Response decoding ---

    #[test]
    fn decode_volume_response() {
        let uuid_str = CHAR_UPSTREAM.to_string();
        let data: Vec<u8> = vec![0xCC, 0x05, 0xAA, 0x00, 0x01, 18];
        let result = decode_response(&uuid_str, &data).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["Volume"], 18);
    }

    #[test]
    fn decode_input_response() {
        let uuid_str = CHAR_UPSTREAM.to_string();
        let data: Vec<u8> = vec![0xCC, 0x05, 0xAA, 0x00, 0x09, 0]; // Source=HdmiArc
        let result = decode_response(&uuid_str, &data).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["CurrentInput"], "HdmiArc");
    }

    #[test]
    fn decode_power_response() {
        let uuid_str = CHAR_UPSTREAM.to_string();
        // Power: 0 = ON (inverted)
        let data: Vec<u8> = vec![0xCC, 0x05, 0xAA, 0x00, 0x0A, 0];
        let result = decode_response(&uuid_str, &data).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["Power"], true);
    }

    #[test]
    fn decode_firmware_version_response() {
        let uuid_str = CHAR_UPSTREAM.to_string();
        let data: Vec<u8> = vec![0xCC, 0x06, 0xAA, 0x01, 0x03, 0x01, 0x01, 0x47];
        let result = decode_response(&uuid_str, &data).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        let fw = parsed["FirmwareVersion"].as_object().unwrap();
        assert_eq!(fw["fw_type"], 1);
        assert_eq!(fw["major"], 1);
        assert_eq!(fw["minor"], 0x47);
    }

    #[test]
    fn decode_unknown_response() {
        let uuid_str = CHAR_UPSTREAM.to_string();
        let data: Vec<u8> = vec![0xCC, 0x05, 0xAA];
        let result = decode_response(&uuid_str, &data).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.get("Unknown").is_some());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn decode_invalid_uuid_returns_error() {
        assert!(decode_response("not-a-uuid", &[0xCC, 0x05, 0xAA, 0x00]).is_err());
    }

    // --- WasmDeviceState ---

    #[test]
    fn device_state_new_is_empty() {
        let state = WasmDeviceState::new();
        assert_eq!(state.volume(), None);
        assert_eq!(state.bass(), None);
        assert_eq!(state.treble(), None);
        assert_eq!(state.mute(), None);
        assert_eq!(state.power(), None);
        assert_eq!(state.input(), None);
        assert_eq!(state.sound_mode(), None);
    }

    #[test]
    fn device_state_apply_volume() {
        let mut state = WasmDeviceState::new();
        state.apply_response(r#"{"Volume": 18}"#).unwrap();
        assert_eq!(state.volume(), Some(18));
    }

    #[test]
    fn device_state_apply_multiple_responses() {
        let mut state = WasmDeviceState::new();
        state.apply_response(r#"{"Volume": 18}"#).unwrap();
        state.apply_response(r#"{"Bass": 10}"#).unwrap();
        state.apply_response(r#"{"Power": true}"#).unwrap();
        state
            .apply_response(r#"{"CurrentInput": "Bluetooth"}"#)
            .unwrap();
        state
            .apply_response(r#"{"CurrentSoundMode": "Movies"}"#)
            .unwrap();

        assert_eq!(state.volume(), Some(18));
        assert_eq!(state.bass(), Some(10));
        assert_eq!(state.power(), Some(true));
        assert_eq!(state.input(), Some("Bluetooth".to_string()));
        assert_eq!(state.sound_mode(), Some("Movies".to_string()));
    }

    #[test]
    fn device_state_apply_mute_and_quiet() {
        let mut state = WasmDeviceState::new();
        state.apply_response(r#"{"MuteState": true}"#).unwrap();
        state.apply_response(r#"{"QuietMode": false}"#).unwrap();

        assert_eq!(state.mute(), Some(true));
        assert_eq!(state.quiet_couch(), Some(false));
    }

    #[test]
    fn device_state_apply_firmware_version() {
        let mut state = WasmDeviceState::new();
        state
            .apply_response(r#"{"FirmwareVersion": {"fw_type": 1, "major": 1, "minor": 71}}"#)
            .unwrap();
        assert_eq!(state.firmware_version(), Some("MCU v1.71".to_string()));
    }

    #[test]
    fn device_state_apply_subwoofer() {
        let mut state = WasmDeviceState::new();
        state
            .apply_response(r#"{"SubwooferConnected": true}"#)
            .unwrap();
        assert_eq!(state.subwoofer_connected(), Some(true));
    }

    #[test]
    fn device_state_apply_rear_volume() {
        let mut state = WasmDeviceState::new();
        state.apply_response(r#"{"RearVolume": 20}"#).unwrap();
        assert_eq!(state.rear_channel_volume(), Some(20));
    }

    #[test]
    fn device_state_apply_covering_and_arm() {
        let mut state = WasmDeviceState::new();
        state.apply_response(r#"{"Covering": 3}"#).unwrap();
        state.apply_response(r#"{"ArmType": 1}"#).unwrap();
        assert_eq!(state.fabric(), Some(3));
        assert_eq!(state.arm_type(), Some(1));
    }

    #[test]
    fn device_state_apply_layout() {
        let mut state = WasmDeviceState::new();
        state.apply_response(r#"{"Layout": 2}"#).unwrap();
        assert_eq!(state.config_shape(), Some("U-Shape".to_string()));
    }

    #[test]
    fn device_state_apply_unknown_is_ignored() {
        let mut state = WasmDeviceState::new();
        state
            .apply_response(
                r#"{"Unknown": {"uuid": "65786365-6c70-6f69-6e74-2e636f6d0001", "data": [1,2,3]}}"#,
            )
            .unwrap();
        // No fields should be set
        assert_eq!(state.volume(), None);
    }

    #[test]
    fn device_state_to_json_roundtrip() {
        let mut state = WasmDeviceState::new();
        state.apply_response(r#"{"Volume": 24}"#).unwrap();
        state.apply_response(r#"{"Power": true}"#).unwrap();

        let json_str = state.to_json().unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["volume"], 24);
        assert_eq!(parsed["power"], true);
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn device_state_apply_invalid_json_returns_error() {
        let mut state = WasmDeviceState::new();
        assert!(state.apply_response("not json").is_err());
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn device_state_apply_non_object_returns_error() {
        let mut state = WasmDeviceState::new();
        assert!(state.apply_response("42").is_err());
    }

    // --- Integration: encode then decode ---

    #[test]
    fn roundtrip_encode_decode_state_update() {
        // Encode a SetVolume command
        let encoded = encode_command(r#"{"SetVolume": 20}"#).unwrap();
        let enc_parsed: Value = serde_json::from_str(&encoded).unwrap();
        assert!(enc_parsed.get("uuid").is_some());
        assert!(enc_parsed.get("data").is_some());

        // Simulate receiving a volume notification
        let uuid_str = CHAR_UPSTREAM.to_string();
        let notification_data: Vec<u8> = vec![0xCC, 0x05, 0xAA, 0x00, 0x01, 20];
        let decoded = decode_response(&uuid_str, &notification_data).unwrap();

        // Apply to state
        let mut state = WasmDeviceState::new();
        state.apply_response(&decoded).unwrap();
        assert_eq!(state.volume(), Some(20));
    }
}
