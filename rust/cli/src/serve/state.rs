//! Application state shared across all web server handlers.

use std::sync::Arc;

use tokio::sync::{broadcast, Mutex};

use libstealthtech_core::ble::scanner::{DiscoveredDevice, Scanner};
use libstealthtech_core::device::StealthTechDevice;

/// Shared application state for the web server.
///
/// Wrapped in `Arc` via axum's `State` extractor so all handlers
/// can access the same device connection and notification channel.
#[derive(Clone)]
pub struct AppState {
    /// The currently connected StealthTech device, if any.
    pub device: Arc<Mutex<Option<StealthTechDevice>>>,
    /// BLE scanner for device discovery.
    pub scanner: Arc<Mutex<Scanner>>,
    /// Cached scan results so `connect` can reuse the same peripheral handles.
    pub scanned_devices: Arc<Mutex<Vec<DiscoveredDevice>>>,
    /// Broadcast channel for forwarding BLE notifications to WebSocket clients.
    pub notifications_tx: broadcast::Sender<String>,
}

impl AppState {
    /// Create a new `AppState` with an initialized BLE scanner and broadcast channel.
    pub async fn new() -> anyhow::Result<Self> {
        let scanner = Scanner::new().await?;
        let (notifications_tx, _) = broadcast::channel(256);

        Ok(Self {
            device: Arc::new(Mutex::new(None)),
            scanner: Arc::new(Mutex::new(scanner)),
            scanned_devices: Arc::new(Mutex::new(Vec::new())),
            notifications_tx,
        })
    }
}
