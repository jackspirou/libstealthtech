use btleplug::api::WriteType;
use tracing::{info, warn};

use crate::ble::connection::{Connection, ConnectionConfig};
use crate::ble::scanner::DiscoveredDevice;
use libstealthtech_protocol::commands::*;
use libstealthtech_protocol::DeviceState;

/// High-level interface to a StealthTech Sound + Charge system.
///
/// This is the primary API for controlling a StealthTech center channel.
/// It wraps the BLE connection and protocol layers into a clean, ergonomic interface.
///
/// # Example
///
/// ```rust,no_run
/// use libstealthtech_core::{Scanner, StealthTechDevice, Input, SoundMode};
///
/// # async fn example() -> anyhow::Result<()> {
/// let scanner = Scanner::new().await?;
/// let devices = scanner.scan(std::time::Duration::from_secs(5)).await?;
///
/// let mut device = StealthTechDevice::connect(devices[0].clone()).await?;
///
/// // Control the system
/// device.set_volume(18).await?;       // 0-36 scale
/// device.set_input(Input::HdmiArc).await?;
/// device.set_sound_mode(SoundMode::Movies).await?;
/// device.set_bass(10).await?;         // 0-20 scale
/// # Ok(())
/// # }
/// ```
pub struct StealthTechDevice {
    connection: Connection,
    device_name: Option<String>,
    device_address: String,
    state: DeviceState,
}

impl StealthTechDevice {
    /// Connect to a discovered StealthTech device with default configuration.
    pub async fn connect(device: DiscoveredDevice) -> anyhow::Result<Self> {
        Self::connect_with_config(device, ConnectionConfig::default()).await
    }

    /// Connect with custom connection configuration.
    pub async fn connect_with_config(
        device: DiscoveredDevice,
        config: ConnectionConfig,
    ) -> anyhow::Result<Self> {
        let mut connection = Connection::new(device.peripheral, config);
        connection.connect().await?;

        Ok(Self {
            connection,
            device_name: device.name,
            device_address: device.address,
            state: DeviceState::default(),
        })
    }

    /// Get the device name.
    pub fn name(&self) -> Option<&str> {
        self.device_name.as_deref()
    }

    /// Get the device BLE address.
    pub fn address(&self) -> &str {
        &self.device_address
    }

    /// Get the current connection state.
    pub fn connection_state(&self) -> tokio::sync::watch::Receiver<crate::ble::connection::ConnectionState> {
        self.connection.state()
    }

    /// Get a snapshot of the last known device state.
    pub fn state(&self) -> &DeviceState {
        &self.state
    }

    /// Get a mutable reference to the device state for applying notification updates.
    pub fn state_mut(&mut self) -> &mut DeviceState {
        &mut self.state
    }

    /// Discover and dump the full GATT profile (for reverse engineering).
    pub async fn discover_gatt(
        &mut self,
    ) -> anyhow::Result<crate::ble::gatt::GattProfile> {
        crate::ble::gatt::discover_gatt_profile(
            &mut self.connection,
            self.device_name.clone(),
            self.device_address.clone(),
        )
        .await
    }

    // ========================================================================
    // Audio Controls (EqControl characteristic)
    // ========================================================================

    /// Set the master volume (0-36).
    pub async fn set_volume(&mut self, volume: u8) -> anyhow::Result<()> {
        self.send_command(Command::SetVolume(volume)).await?;
        self.state.volume = Some(volume);
        Ok(())
    }

    /// Mute or unmute the system.
    pub async fn set_mute(&mut self, muted: bool) -> anyhow::Result<()> {
        self.send_command(Command::SetMute(muted)).await?;
        self.state.mute = Some(muted);
        Ok(())
    }

    /// Set the audio input source.
    pub async fn set_input(&mut self, input: Input) -> anyhow::Result<()> {
        self.send_command(Command::SetInput(input)).await?;
        self.state.input = Some(input);
        Ok(())
    }

    /// Set the sound preset/mode.
    pub async fn set_sound_mode(&mut self, mode: SoundMode) -> anyhow::Result<()> {
        self.send_command(Command::SetSoundMode(mode)).await?;
        self.state.sound_mode = Some(mode);
        Ok(())
    }

    // ========================================================================
    // EQ / Balance
    // ========================================================================

    /// Set bass level (0-20).
    pub async fn set_bass(&mut self, level: u8) -> anyhow::Result<()> {
        self.send_command(Command::SetBass(level)).await?;
        self.state.bass = Some(level);
        Ok(())
    }

    /// Set treble level (0-20).
    pub async fn set_treble(&mut self, level: u8) -> anyhow::Result<()> {
        self.send_command(Command::SetTreble(level)).await?;
        self.state.treble = Some(level);
        Ok(())
    }

    /// Set center channel volume offset (0-30).
    pub async fn set_center_volume(&mut self, level: u8) -> anyhow::Result<()> {
        self.send_command(Command::SetCenterVolume(level)).await?;
        self.state.center_volume = Some(level);
        Ok(())
    }

    /// Set L/R speaker balance (0-100, 50=center).
    pub async fn set_balance(&mut self, balance: u8) -> anyhow::Result<()> {
        self.send_command(Command::SetBalance(balance)).await?;
        self.state.balance = Some(balance);
        Ok(())
    }

    /// Set rear channel volume (0-30).
    pub async fn set_rear_channel_volume(&mut self, level: u8) -> anyhow::Result<()> {
        self.send_command(Command::SetRearChannelVolume(level)).await?;
        self.state.rear_channel_volume = Some(level);
        Ok(())
    }

    // ========================================================================
    // Speaker Configuration
    // ========================================================================

    /// Enable or disable surround speakers.
    pub async fn set_surround(&mut self, enabled: bool) -> anyhow::Result<()> {
        self.send_command(Command::SetSurroundEnabled(enabled)).await?;
        self.state.surround_enabled = Some(enabled);
        Ok(())
    }

    /// Toggle Quiet Couch Mode.
    pub async fn set_quiet_couch(&mut self, enabled: bool) -> anyhow::Result<()> {
        self.send_command(Command::SetQuietCouch(enabled)).await?;
        self.state.quiet_couch = Some(enabled);
        Ok(())
    }

    // ========================================================================
    // Tuning
    // ========================================================================

    /// Set the fabric covering type for acoustic tuning (byte ID).
    pub async fn set_fabric(&mut self, fabric_id: u8) -> anyhow::Result<()> {
        self.send_command(Command::SetFabric(fabric_id)).await?;
        self.state.fabric = Some(fabric_id);
        Ok(())
    }

    /// Set the Sactionals configuration shape for surround calibration.
    pub async fn set_config_shape(&mut self, shape: ConfigShape) -> anyhow::Result<()> {
        self.send_command(Command::SetConfigShape(shape)).await?;
        self.state.config_shape = Some(shape);
        Ok(())
    }

    /// Set the arm type.
    pub async fn set_arm_type(&mut self, arm_type: u8) -> anyhow::Result<()> {
        self.send_command(Command::SetArmType(arm_type)).await?;
        self.state.arm_type = Some(arm_type);
        Ok(())
    }

    // ========================================================================
    // System
    // ========================================================================

    /// Power on or enter standby.
    pub async fn set_power(&mut self, on: bool) -> anyhow::Result<()> {
        self.send_command(Command::SetPower(on)).await?;
        self.state.power = Some(on);
        Ok(())
    }

    /// Send a play/pause command via Bluetooth media control.
    pub async fn set_play_pause(&mut self, value: u8) -> anyhow::Result<()> {
        self.send_command(Command::SetPlayPause(value)).await
    }

    /// Send a skip track command via Bluetooth media control (0=forward, 1=back).
    pub async fn set_skip(&mut self, value: u8) -> anyhow::Result<()> {
        self.send_command(Command::SetSkip(value)).await
    }

    /// Request a full device state refresh (responses arrive via notifications).
    pub async fn request_state(&mut self) -> anyhow::Result<()> {
        self.send_command(Command::GetState).await
    }

    /// Request firmware version info (response arrives via notification).
    pub async fn request_firmware_version(&mut self) -> anyhow::Result<()> {
        self.send_command(Command::GetFirmwareVersion).await
    }

    /// Disconnect from the device.
    pub async fn disconnect(&mut self) -> anyhow::Result<()> {
        self.connection.disconnect().await
    }

    /// Subscribe to all notifiable characteristics and return the notification stream.
    ///
    /// Unlike `monitor_notifications()`, this returns the stream for external consumption
    /// (e.g., by the web server's WebSocket handler).
    pub async fn start_notifications(
        &mut self,
    ) -> anyhow::Result<
        std::pin::Pin<
            Box<dyn futures::Stream<Item = btleplug::api::ValueNotification> + Send>,
        >,
    > {
        let chars = self.connection.characteristics();
        let notifiable: Vec<_> = chars
            .iter()
            .filter(|c| {
                c.properties
                    .contains(btleplug::api::CharPropFlags::NOTIFY)
                    || c.properties
                        .contains(btleplug::api::CharPropFlags::INDICATE)
            })
            .collect();

        info!(
            count = notifiable.len(),
            "Subscribing to notifiable characteristics"
        );

        for char in &notifiable {
            if let Err(e) = self.connection.subscribe(char).await {
                warn!(uuid = %char.uuid, error = %e, "Failed to subscribe");
            }
        }

        self.connection.notifications().await
    }

    /// Subscribe to all notifiable characteristics and log traffic.
    ///
    /// This subscribes to the UpStream characteristic and logs all incoming
    /// notifications, decoding them into typed responses where possible.
    pub async fn monitor_notifications(&mut self) -> anyhow::Result<()> {
        use futures::StreamExt;

        let chars = self.connection.characteristics();
        let notifiable: Vec<_> = chars
            .iter()
            .filter(|c| {
                c.properties
                    .contains(btleplug::api::CharPropFlags::NOTIFY)
                    || c.properties
                        .contains(btleplug::api::CharPropFlags::INDICATE)
            })
            .collect();

        info!(
            count = notifiable.len(),
            "Subscribing to notifiable characteristics"
        );

        for char in &notifiable {
            if let Err(e) = self.connection.subscribe(char).await {
                warn!(uuid = %char.uuid, error = %e, "Failed to subscribe");
            }
        }

        let mut stream = self.connection.notifications().await?;

        info!("Monitoring notifications. Use the official app or remote to generate traffic...");

        while let Some(notification) = stream.next().await {
            let response = Response::decode(
                notification.uuid,
                &notification.value,
            );

            match response {
                Response::Unknown {
                    characteristic_uuid,
                    data,
                } => {
                    println!(
                        "[NOTIFY] UUID: {} | Len: {} | Hex: {}",
                        characteristic_uuid,
                        data.len(),
                        hex::encode(&data),
                    );
                }
                other => {
                    println!("[NOTIFY] {}", other);
                }
            }
        }

        Ok(())
    }

    /// Internal: send a command via BLE.
    ///
    /// Writes are sent **without response** as confirmed by the protocol.
    async fn send_command(&mut self, cmd: Command) -> anyhow::Result<()> {
        let (uuid, data) = cmd.encode()?;
        let chars = self.connection.characteristics();
        let char = chars
            .iter()
            .find(|c| c.uuid == uuid)
            .ok_or_else(|| {
                anyhow::anyhow!("Characteristic {} not found on device", uuid)
            })?
            .clone();

        self.connection
            .write(&char, &data, WriteType::WithoutResponse)
            .await?;
        Ok(())
    }
}
