//! BLE connection management with robust reconnection logic.
//!
//! The #1 complaint about StealthTech is BLE disconnection. This module implements
//! aggressive retry logic, connection health monitoring, and automatic reconnection
//! to provide a far more reliable experience than the official app.

use std::fmt;
use std::time::Duration;

use btleplug::api::{Characteristic, Peripheral as _, WriteType};
use btleplug::platform::Peripheral;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

/// Connection state observable by consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempt: u32 },
}

impl fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionState::Disconnected => write!(f, "Disconnected"),
            ConnectionState::Connecting => write!(f, "Connecting"),
            ConnectionState::Connected => write!(f, "Connected"),
            ConnectionState::Reconnecting { attempt } => write!(f, "Reconnecting (attempt {})", attempt),
        }
    }
}

/// Configuration for connection behavior.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Maximum number of reconnection attempts before giving up.
    pub max_reconnect_attempts: u32,
    /// Base delay between reconnection attempts (exponential backoff applied).
    pub reconnect_base_delay: Duration,
    /// Maximum delay between reconnection attempts.
    pub reconnect_max_delay: Duration,
    /// Timeout for initial connection.
    pub connect_timeout: Duration,
    /// Timeout for GATT service discovery after connection.
    pub discovery_timeout: Duration,
    /// Interval for connection health checks (keepalive reads).
    pub keepalive_interval: Duration,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            max_reconnect_attempts: 10,
            reconnect_base_delay: Duration::from_millis(500),
            reconnect_max_delay: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(15),
            discovery_timeout: Duration::from_secs(10),
            keepalive_interval: Duration::from_secs(5),
        }
    }
}

/// Managed BLE connection to a StealthTech center channel.
///
/// Provides reliable read/write operations with automatic reconnection,
/// exponential backoff, and connection state observability.
pub struct Connection {
    peripheral: Peripheral,
    config: ConnectionConfig,
    state_tx: watch::Sender<ConnectionState>,
    state_rx: watch::Receiver<ConnectionState>,
}

impl Connection {
    /// Create a new connection manager for the given peripheral.
    pub fn new(peripheral: Peripheral, config: ConnectionConfig) -> Self {
        let (state_tx, state_rx) = watch::channel(ConnectionState::Disconnected);
        Self {
            peripheral,
            config,
            state_tx,
            state_rx,
        }
    }

    /// Get a receiver for connection state changes.
    pub fn state(&self) -> watch::Receiver<ConnectionState> {
        self.state_rx.clone()
    }

    /// Establish the initial BLE connection and discover GATT services.
    pub async fn connect(&mut self) -> anyhow::Result<()> {
        self.state_tx.send_replace(ConnectionState::Connecting);

        info!("Connecting to StealthTech center channel...");

        // Connect with timeout
        tokio::time::timeout(self.config.connect_timeout, self.peripheral.connect())
            .await
            .map_err(|_| anyhow::anyhow!("Connection timed out"))??;

        // Discover GATT services
        tokio::time::timeout(
            self.config.discovery_timeout,
            self.peripheral.discover_services(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Service discovery timed out"))??;

        self.state_tx.send_replace(ConnectionState::Connected);
        info!("Connected and services discovered");

        Ok(())
    }

    /// Disconnect from the device.
    pub async fn disconnect(&mut self) -> anyhow::Result<()> {
        self.peripheral.disconnect().await?;
        self.state_tx.send_replace(ConnectionState::Disconnected);
        info!("Disconnected from StealthTech");
        Ok(())
    }

    /// Check if the peripheral reports as connected.
    pub async fn is_connected(&self) -> bool {
        self.peripheral.is_connected().await.unwrap_or(false)
    }

    /// Get all discovered GATT characteristics.
    pub fn characteristics(&self) -> std::collections::BTreeSet<Characteristic> {
        self.peripheral.characteristics()
    }

    /// Read a characteristic value, with automatic reconnection on failure.
    pub async fn read(&mut self, characteristic: &Characteristic) -> anyhow::Result<Vec<u8>> {
        match self.peripheral.read(characteristic).await {
            Ok(data) => {
                debug!(
                    uuid = %characteristic.uuid,
                    len = data.len(),
                    data = %hex::encode(&data),
                    "BLE read success"
                );
                Ok(data)
            }
            Err(e) => {
                warn!(error = %e, uuid = %characteristic.uuid, "BLE read failed, attempting reconnect");
                self.reconnect().await?;
                let data = self.peripheral.read(characteristic).await?;
                Ok(data)
            }
        }
    }

    /// Write to a characteristic, with automatic reconnection on failure.
    pub async fn write(
        &mut self,
        characteristic: &Characteristic,
        data: &[u8],
        write_type: WriteType,
    ) -> anyhow::Result<()> {
        debug!(
            uuid = %characteristic.uuid,
            data = %hex::encode(data),
            write_type = ?write_type,
            "BLE write"
        );

        match self.peripheral.write(characteristic, data, write_type).await {
            Ok(()) => Ok(()),
            Err(e) => {
                warn!(error = %e, uuid = %characteristic.uuid, "BLE write failed, attempting reconnect");
                self.reconnect().await?;
                self.peripheral
                    .write(characteristic, data, write_type)
                    .await?;
                Ok(())
            }
        }
    }

    /// Subscribe to notifications on a characteristic.
    pub async fn subscribe(&self, characteristic: &Characteristic) -> anyhow::Result<()> {
        self.peripheral.subscribe(characteristic).await?;
        debug!(uuid = %characteristic.uuid, "Subscribed to notifications");
        Ok(())
    }

    /// Get the notification stream from the peripheral.
    pub async fn notifications(
        &self,
    ) -> anyhow::Result<std::pin::Pin<Box<dyn futures::Stream<Item = btleplug::api::ValueNotification> + Send>>>
    {
        Ok(self.peripheral.notifications().await?)
    }

    /// Get a reference to the underlying peripheral (for advanced use).
    pub fn peripheral(&self) -> &Peripheral {
        &self.peripheral
    }

    /// Attempt to reconnect with exponential backoff.
    async fn reconnect(&mut self) -> anyhow::Result<()> {
        for attempt in 1..=self.config.max_reconnect_attempts {
            self.state_tx
                .send_replace(ConnectionState::Reconnecting { attempt });

            let delay = std::cmp::min(
                self.config.reconnect_base_delay * 2u32.saturating_pow(attempt - 1),
                self.config.reconnect_max_delay,
            );

            warn!(attempt, max = self.config.max_reconnect_attempts, ?delay, "Reconnecting...");

            tokio::time::sleep(delay).await;

            // Try to disconnect cleanly first (ignore errors)
            let _ = self.peripheral.disconnect().await;
            tokio::time::sleep(Duration::from_millis(200)).await;

            match tokio::time::timeout(self.config.connect_timeout, self.peripheral.connect()).await
            {
                Ok(Ok(())) => {
                    // Re-discover services after reconnection
                    if let Ok(Ok(())) = tokio::time::timeout(
                        self.config.discovery_timeout,
                        self.peripheral.discover_services(),
                    )
                    .await
                    {
                        self.state_tx.send_replace(ConnectionState::Connected);
                        info!(attempt, "Reconnected successfully");
                        return Ok(());
                    }
                }
                Ok(Err(e)) => {
                    error!(attempt, error = %e, "Reconnection attempt failed");
                }
                Err(_) => {
                    error!(attempt, "Reconnection attempt timed out");
                }
            }
        }

        self.state_tx.send_replace(ConnectionState::Disconnected);
        Err(anyhow::anyhow!(
            "Failed to reconnect after {} attempts",
            self.config.max_reconnect_attempts
        ))
    }
}
