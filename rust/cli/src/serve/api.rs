//! REST API endpoint handlers for the StealthTech web server.

use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use libstealthtech_core::device::StealthTechDevice;
use libstealthtech_core::protocol::characteristics::{
    LATEST_DSP_VERSION, LATEST_EQ_VERSION, LATEST_MCU_VERSION,
};
use libstealthtech_core::protocol::commands::{ConfigShape, Input, SoundMode};

use super::state::AppState;

// ============================================================================
// Response types
// ============================================================================

/// JSON representation of the current device state for API responses.
#[derive(Serialize)]
struct DeviceStateResponse {
    connected: bool,
    name: Option<String>,
    address: Option<String>,
    volume: Option<u8>,
    bass: Option<u8>,
    treble: Option<u8>,
    center_volume: Option<u8>,
    rear_channel_volume: Option<u8>,
    balance: Option<u8>,
    mute: Option<bool>,
    power: Option<bool>,
    quiet_couch: Option<bool>,
    subwoofer_connected: Option<bool>,
    config_shape: Option<String>,
    input: Option<String>,
    sound_mode: Option<String>,
    firmware: Option<FirmwareStatusResponse>,
}

/// Per-component firmware version info.
#[derive(Serialize)]
struct FirmwareComponentResponse {
    current: Option<String>,
    latest: String,
    up_to_date: Option<bool>,
}

/// Firmware version status for API responses.
#[derive(Serialize)]
struct FirmwareStatusResponse {
    mcu: FirmwareComponentResponse,
    dsp: FirmwareComponentResponse,
    eq: FirmwareComponentResponse,
    update_available: bool,
}

impl DeviceStateResponse {
    fn from_device(device: &StealthTechDevice) -> Self {
        let state = device.state();

        let firmware = if state.mcu_version.is_some()
            || state.dsp_version.is_some()
            || state.eq_version.is_some()
        {
            Some(FirmwareStatusResponse {
                mcu: FirmwareComponentResponse {
                    current: state.mcu_version.map(|v| format!("{v}")),
                    latest: format!("{LATEST_MCU_VERSION}"),
                    up_to_date: state
                        .mcu_version
                        .map(|v| v.is_at_least(&LATEST_MCU_VERSION)),
                },
                dsp: FirmwareComponentResponse {
                    current: state.dsp_version.map(|v| format!("{v}")),
                    latest: format!("{LATEST_DSP_VERSION}"),
                    up_to_date: state
                        .dsp_version
                        .map(|v| v.is_at_least(&LATEST_DSP_VERSION)),
                },
                eq: FirmwareComponentResponse {
                    current: state.eq_version.map(|v| format!("{v}")),
                    latest: format!("{LATEST_EQ_VERSION}"),
                    up_to_date: state.eq_version.map(|v| v.is_at_least(&LATEST_EQ_VERSION)),
                },
                update_available: state.firmware_update_available().unwrap_or(false),
            })
        } else {
            None
        };

        Self {
            connected: true,
            name: device.name().map(String::from),
            address: Some(device.address().to_string()),
            volume: state.volume,
            bass: state.bass,
            treble: state.treble,
            center_volume: state.center_volume,
            rear_channel_volume: state.rear_channel_volume,
            balance: state.balance,
            mute: state.mute,
            power: state.power,
            quiet_couch: state.quiet_couch,
            subwoofer_connected: state.subwoofer_connected,
            config_shape: state.config_shape.map(|s| format!("{s}")),
            input: state.input.map(|i| format!("{}", i)),
            sound_mode: state.sound_mode.map(|m| format!("{}", m)),
            firmware,
        }
    }

    fn disconnected() -> Self {
        Self {
            connected: false,
            name: None,
            address: None,
            volume: None,
            bass: None,
            treble: None,
            center_volume: None,
            rear_channel_volume: None,
            balance: None,
            mute: None,
            power: None,
            quiet_couch: None,
            subwoofer_connected: None,
            config_shape: None,
            input: None,
            sound_mode: None,
            firmware: None,
        }
    }
}

/// JSON payload for discovered BLE devices.
#[derive(Serialize)]
struct DiscoveredDeviceResponse {
    name: Option<String>,
    address: String,
    rssi: Option<i16>,
}

// ============================================================================
// Request payloads
// ============================================================================

#[derive(Deserialize)]
struct ConnectRequest {
    address: String,
}

#[derive(Deserialize)]
struct ValueU8Request {
    value: u8,
}

#[derive(Deserialize)]
struct ValueBoolRequest {
    value: bool,
}

#[derive(Deserialize)]
struct InputRequest {
    value: String,
}

#[derive(Deserialize)]
struct ModeRequest {
    value: String,
}

#[derive(Deserialize)]
struct ShapeRequest {
    value: String,
}

// ============================================================================
// Routes
// ============================================================================

/// Build the API router with all REST endpoints.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/state", get(get_state))
        .route("/devices", get(scan_devices))
        .route("/connect", post(connect_device))
        .route("/disconnect", post(disconnect_device))
        .route("/volume", post(set_volume))
        .route("/bass", post(set_bass))
        .route("/treble", post(set_treble))
        .route("/mute", post(set_mute))
        .route("/input", post(set_input))
        .route("/mode", post(set_mode))
        .route("/power", post(set_power))
        .route("/quiet-couch", post(set_quiet_couch))
        .route("/center-volume", post(set_center_volume))
        .route("/rear-volume", post(set_rear_volume))
        .route("/balance", post(set_balance))
        .route("/config-shape", post(set_config_shape))
        .route("/play-pause", post(set_play_pause))
        .route("/skip", post(set_skip))
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/state -- return the current device state as JSON.
async fn get_state(State(state): State<AppState>) -> impl IntoResponse {
    let device_guard = state.device.lock().await;
    match device_guard.as_ref() {
        Some(device) => Json(DeviceStateResponse::from_device(device)).into_response(),
        None => Json(DeviceStateResponse::disconnected()).into_response(),
    }
}

/// GET /api/devices -- scan for nearby StealthTech devices.
async fn scan_devices(State(state): State<AppState>) -> impl IntoResponse {
    let scanner = state.scanner.lock().await;
    match scanner.scan(Duration::from_secs(5)).await {
        Ok(devices) => {
            let response: Vec<DiscoveredDeviceResponse> = devices
                .iter()
                .map(|d| DiscoveredDeviceResponse {
                    name: d.name.clone(),
                    address: d.address.clone(),
                    rssi: d.rssi,
                })
                .collect();

            // Cache scan results so connect can reuse the same peripheral handles
            let mut cached = state.scanned_devices.lock().await;
            *cached = devices;

            Json(response).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{}", e) })),
        )
            .into_response(),
    }
}

/// POST /api/connect -- connect to a device by BLE address.
async fn connect_device(
    State(state): State<AppState>,
    Json(payload): Json<ConnectRequest>,
) -> impl IntoResponse {
    // Look up peripheral from cached scan results (avoids a second scan
    // that would create new Peripheral objects without CoreBluetooth context).
    let mut cached = state.scanned_devices.lock().await;
    let idx = cached
        .iter()
        .position(|d| d.address.to_lowercase() == payload.address.to_lowercase());
    let discovered = match idx {
        Some(i) => cached.remove(i),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Device not found in scan results. Please scan again." })),
            )
                .into_response();
        }
    };
    drop(cached);

    let mut device = match StealthTechDevice::connect(discovered).await {
        Ok(d) => d,
        Err(e) => {
            let msg = format!("{}", e);
            let status = if msg.contains("timed out") {
                StatusCode::GATEWAY_TIMEOUT
            } else {
                StatusCode::BAD_GATEWAY
            };
            return (
                status,
                Json(serde_json::json!({ "error": format!("Connection failed: {}", e) })),
            )
                .into_response();
        }
    };

    // Start notifications, request device state, collect initial burst, then
    // spawn a background task to keep state in sync and forward to WebSocket.
    match device.start_notifications().await {
        Ok(stream) => {
            use futures::StreamExt;
            use libstealthtech_core::protocol::commands::Response;

            let tx = state.notifications_tx.clone();
            let mut stream = stream;

            // Request a full state dump and firmware version from the device
            if let Err(e) = device.request_state().await {
                tracing::warn!(error = %e, "Failed to request initial state");
            }
            if let Err(e) = device.request_firmware_version().await {
                tracing::warn!(error = %e, "Failed to request firmware version");
            }

            // Collect the notification burst (individual field updates) for up to 500ms
            let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
            while let Ok(Some(notification)) =
                tokio::time::timeout_at(deadline, stream.next()).await
            {
                let response = Response::decode(notification.uuid, &notification.value);
                device.state_mut().apply_response(&response);
                let msg = serde_json::json!({
                    "type": "notification",
                    "uuid": notification.uuid.to_string(),
                    "hex": hex::encode(&notification.value),
                    "decoded": format!("{}", response),
                });
                let _ = tx.send(msg.to_string());
            }

            // Spawn ongoing forwarding task that also keeps device state in sync
            let device_arc = state.device.clone();
            tokio::spawn(async move {
                while let Some(notification) = stream.next().await {
                    let response = Response::decode(notification.uuid, &notification.value);
                    // Update device state from external changes (remote, official app)
                    {
                        let mut guard = device_arc.lock().await;
                        if let Some(ref mut dev) = *guard {
                            dev.state_mut().apply_response(&response);
                        }
                    }
                    let msg = serde_json::json!({
                        "type": "notification",
                        "uuid": notification.uuid.to_string(),
                        "hex": hex::encode(&notification.value),
                        "decoded": format!("{}", response),
                    });
                    let _ = tx.send(msg.to_string());
                }
            });
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to start notifications");
        }
    }

    let response = DeviceStateResponse::from_device(&device);
    let mut device_guard = state.device.lock().await;
    *device_guard = Some(device);

    Json(response).into_response()
}

/// POST /api/disconnect -- disconnect from the current device.
async fn disconnect_device(State(state): State<AppState>) -> impl IntoResponse {
    let mut device_guard = state.device.lock().await;
    if let Some(ref mut device) = *device_guard {
        if let Err(e) = device.disconnect().await {
            tracing::warn!(error = %e, "Error during disconnect");
        }
    }
    *device_guard = None;
    Json(DeviceStateResponse::disconnected()).into_response()
}

/// POST /api/volume -- set volume level (0-36).
async fn set_volume(
    State(state): State<AppState>,
    Json(payload): Json<ValueU8Request>,
) -> impl IntoResponse {
    with_device(&state, |device| {
        Box::pin(async move { device.set_volume(payload.value).await })
    })
    .await
}

/// POST /api/bass -- set bass level (0-20).
async fn set_bass(
    State(state): State<AppState>,
    Json(payload): Json<ValueU8Request>,
) -> impl IntoResponse {
    with_device(&state, |device| {
        Box::pin(async move { device.set_bass(payload.value).await })
    })
    .await
}

/// POST /api/treble -- set treble level (0-20).
async fn set_treble(
    State(state): State<AppState>,
    Json(payload): Json<ValueU8Request>,
) -> impl IntoResponse {
    with_device(&state, |device| {
        Box::pin(async move { device.set_treble(payload.value).await })
    })
    .await
}

/// POST /api/mute -- mute or unmute.
async fn set_mute(
    State(state): State<AppState>,
    Json(payload): Json<ValueBoolRequest>,
) -> impl IntoResponse {
    with_device(&state, |device| {
        Box::pin(async move { device.set_mute(payload.value).await })
    })
    .await
}

/// POST /api/input -- set input source (hdmi/bluetooth/aux/optical).
async fn set_input(
    State(state): State<AppState>,
    Json(payload): Json<InputRequest>,
) -> impl IntoResponse {
    let input = match payload.value.to_lowercase().as_str() {
        "hdmi" | "hdmi_arc" | "hdmiarc" | "hdmi arc" => Input::HdmiArc,
        "bluetooth" | "bt" => Input::Bluetooth,
        "aux" | "auxiliary" => Input::Aux,
        "optical" | "toslink" => Input::Optical,
        other => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Unknown input: {}", other) })),
            )
                .into_response();
        }
    };

    with_device(&state, |device| {
        Box::pin(async move { device.set_input(input).await })
    })
    .await
}

/// POST /api/mode -- set sound mode (movies/music/tv/news/manual).
async fn set_mode(
    State(state): State<AppState>,
    Json(payload): Json<ModeRequest>,
) -> impl IntoResponse {
    let mode = match payload.value.to_lowercase().as_str() {
        "movies" | "movie" => SoundMode::Movies,
        "music" => SoundMode::Music,
        "tv" | "television" => SoundMode::Tv,
        "news" | "dialog" => SoundMode::News,
        "manual" => SoundMode::Manual,
        other => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Unknown mode: {}", other) })),
            )
                .into_response();
        }
    };

    with_device(&state, |device| {
        Box::pin(async move { device.set_sound_mode(mode).await })
    })
    .await
}

/// POST /api/power -- power on or standby.
async fn set_power(
    State(state): State<AppState>,
    Json(payload): Json<ValueBoolRequest>,
) -> impl IntoResponse {
    with_device(&state, |device| {
        Box::pin(async move { device.set_power(payload.value).await })
    })
    .await
}

/// POST /api/quiet-couch -- toggle Quiet Couch Mode.
async fn set_quiet_couch(
    State(state): State<AppState>,
    Json(payload): Json<ValueBoolRequest>,
) -> impl IntoResponse {
    with_device(&state, |device| {
        Box::pin(async move { device.set_quiet_couch(payload.value).await })
    })
    .await
}

/// POST /api/center-volume -- set center channel volume (0-30).
async fn set_center_volume(
    State(state): State<AppState>,
    Json(payload): Json<ValueU8Request>,
) -> impl IntoResponse {
    with_device(&state, |device| {
        Box::pin(async move { device.set_center_volume(payload.value).await })
    })
    .await
}

/// POST /api/rear-volume -- set rear channel volume (0-30).
async fn set_rear_volume(
    State(state): State<AppState>,
    Json(payload): Json<ValueU8Request>,
) -> impl IntoResponse {
    with_device(&state, |device| {
        Box::pin(async move { device.set_rear_channel_volume(payload.value).await })
    })
    .await
}

/// POST /api/balance -- set L/R balance (0-100, 50=center).
async fn set_balance(
    State(state): State<AppState>,
    Json(payload): Json<ValueU8Request>,
) -> impl IntoResponse {
    with_device(&state, |device| {
        Box::pin(async move { device.set_balance(payload.value).await })
    })
    .await
}

/// POST /api/config-shape -- set couch configuration shape.
async fn set_config_shape(
    State(state): State<AppState>,
    Json(payload): Json<ShapeRequest>,
) -> impl IntoResponse {
    let shape = match payload.value.to_lowercase().as_str() {
        "straight" => ConfigShape::Straight,
        "lshape" | "l-shape" => ConfigShape::LShape,
        "ushape" | "u-shape" => ConfigShape::UShape,
        "pit" => ConfigShape::Pit,
        other => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Unknown shape: {}", other) })),
            )
                .into_response();
        }
    };

    with_device(&state, |device| {
        Box::pin(async move { device.set_config_shape(shape).await })
    })
    .await
}

/// POST /api/play-pause -- send play/pause media command.
async fn set_play_pause(
    State(state): State<AppState>,
    Json(payload): Json<ValueU8Request>,
) -> impl IntoResponse {
    with_device(&state, |device| {
        Box::pin(async move { device.set_play_pause(payload.value).await })
    })
    .await
}

/// POST /api/skip -- send skip track media command (0=forward, 1=back).
async fn set_skip(
    State(state): State<AppState>,
    Json(payload): Json<ValueU8Request>,
) -> impl IntoResponse {
    with_device(&state, |device| {
        Box::pin(async move { device.set_skip(payload.value).await })
    })
    .await
}

// ============================================================================
// Helper
// ============================================================================

/// Lock the device mutex, run an async operation, and return the updated state.
///
/// Returns 400 if no device is connected, 500 on operation failure, or 200
/// with the updated device state JSON on success.
async fn with_device<F>(state: &AppState, op: F) -> axum::response::Response
where
    F: for<'a> FnOnce(
        &'a mut StealthTechDevice,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + 'a>,
    >,
{
    let mut device_guard = state.device.lock().await;
    match device_guard.as_mut() {
        Some(device) => match op(device).await {
            Ok(()) => Json(DeviceStateResponse::from_device(device)).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("{}", e) })),
            )
                .into_response(),
        },
        None => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "No device connected" })),
        )
            .into_response(),
    }
}
