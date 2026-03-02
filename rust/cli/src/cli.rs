//! stealthtech: Command-line tool for controlling and reverse-engineering StealthTech systems.

mod serve;
mod sniff;

use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use libstealthtech_core::ble::scanner::Scanner;
use libstealthtech_core::device::StealthTechDevice;
use libstealthtech_core::protocol::commands::*;

#[derive(Parser)]
#[command(
    name = "stealthtech",
    about = "Open-source CLI for Lovesac StealthTech Sound + Charge",
    version,
    long_about = "Control your StealthTech system via BLE.\n\
                   Volume range: 0-36 | Bass/Treble: 0-20 | Center/Rear: 0-30 | Balance: 0-100"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// BLE scan duration in seconds.
    #[arg(long, default_value = "5")]
    scan_timeout: u64,

    /// Target device BLE address (skip scan if known).
    #[arg(long)]
    address: Option<String>,

    /// Enable verbose logging (set RUST_LOG for fine control).
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan for StealthTech devices.
    Scan,

    /// Set master volume (0-36).
    Volume {
        #[arg(value_parser = clap::value_parser!(u8).range(0..=36))]
        level: u8,
    },

    /// Mute or unmute the system.
    Mute {
        /// "on" to mute, "off" to unmute.
        #[arg(value_enum)]
        state: OnOff,
    },

    /// Select audio input source.
    Input {
        #[arg(value_enum)]
        source: InputArg,
    },

    /// Set sound preset/mode.
    Mode {
        #[arg(value_enum)]
        mode: ModeArg,
    },

    /// Set bass level (0-20).
    Bass {
        #[arg(value_parser = clap::value_parser!(u8).range(0..=20))]
        level: u8,
    },

    /// Set treble level (0-20).
    Treble {
        #[arg(value_parser = clap::value_parser!(u8).range(0..=20))]
        level: u8,
    },

    /// Toggle Quiet Couch Mode.
    QuietCouch {
        #[arg(value_enum)]
        state: OnOff,
    },

    /// Power on or enter standby.
    Power {
        #[arg(value_enum)]
        state: OnOff,
    },

    /// Get device info (firmware, model, etc).
    Info,

    /// BLE protocol reverse engineering tools.
    Sniff {
        #[command(subcommand)]
        command: sniff::SniffCommands,
    },

    /// Start a web server with a browser-based remote control UI.
    Serve {
        /// Port to listen on.
        #[arg(long, default_value = "3000")]
        port: u16,
    },

    /// Export static Web Bluetooth UI files (no server needed).
    ///
    /// Outputs a self-contained static website that works with Web Bluetooth
    /// directly in Chrome. Can be opened from file:// or hosted on any
    /// static server.
    Export {
        /// Output directory for static files.
        #[arg(long, default_value = ".")]
        output: PathBuf,
    },
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum OnOff {
    On,
    Off,
}

impl OnOff {
    fn is_on(&self) -> bool {
        matches!(self, OnOff::On)
    }
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum InputArg {
    Hdmi,
    Optical,
    Bluetooth,
    Aux,
}

impl From<InputArg> for Input {
    fn from(a: InputArg) -> Self {
        match a {
            InputArg::Hdmi => Input::HdmiArc,
            InputArg::Optical => Input::Optical,
            InputArg::Bluetooth => Input::Bluetooth,
            InputArg::Aux => Input::Aux,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum ModeArg {
    Movies,
    Music,
    Tv,
    News,
    Manual,
}

impl From<ModeArg> for SoundMode {
    fn from(a: ModeArg) -> Self {
        match a {
            ModeArg::Movies => SoundMode::Movies,
            ModeArg::Music => SoundMode::Music,
            ModeArg::Tv => SoundMode::Tv,
            ModeArg::News => SoundMode::News,
            ModeArg::Manual => SoundMode::Manual,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose {
        "libstealthtech_core=debug,btleplug=debug"
    } else {
        "libstealthtech_core=info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .init();

    match cli.command {
        Commands::Scan => {
            let scanner = Scanner::new().await?;
            let devices = scanner.scan(Duration::from_secs(cli.scan_timeout)).await?;

            if devices.is_empty() {
                println!("No StealthTech devices found.");
                println!("Make sure your center channel is powered on and nearby.");
            } else {
                println!("Found {} StealthTech device(s):\n", devices.len());
                for device in &devices {
                    println!("  Name:    {}", device.name.as_deref().unwrap_or("Unknown"));
                    println!("  Address: {}", device.address);
                    println!(
                        "  RSSI:    {} dBm",
                        device
                            .rssi
                            .map(|r| r.to_string())
                            .unwrap_or_else(|| "N/A".into())
                    );
                    println!();
                }
            }
        }

        Commands::Info => {
            let mut device = find_and_connect(&cli).await?;
            let profile = device.discover_gatt().await?;

            println!("StealthTech Device Information:");
            println!("  Name:    {}", device.name().unwrap_or("Unknown"));
            println!("  Address: {}", device.address());

            for service in &profile.services {
                for char in &service.characteristics {
                    if let Some(ref desc) = char.description {
                        if let Some(ref val) = char.value_utf8 {
                            println!("  {}: {}", desc, val);
                        } else if let Some(ref val) = char.value_hex {
                            println!("  {} (hex): {}", desc, val);
                        }
                    }
                }
            }

            device.disconnect().await?;
        }

        Commands::Volume { level } => {
            let mut device = find_and_connect(&cli).await?;
            device.set_volume(level).await?;
            println!("Volume set to {}/36", level);
            device.disconnect().await?;
        }

        Commands::Mute { state } => {
            let mut device = find_and_connect(&cli).await?;
            device.set_mute(state.is_on()).await?;
            println!("Mute {}", if state.is_on() { "on" } else { "off" });
            device.disconnect().await?;
        }

        Commands::Input { source } => {
            let input: Input = source.into();
            let mut device = find_and_connect(&cli).await?;
            device.set_input(input).await?;
            println!("Input set to {}", input);
            device.disconnect().await?;
        }

        Commands::Mode { mode } => {
            let sound_mode: SoundMode = mode.into();
            let mut device = find_and_connect(&cli).await?;
            device.set_sound_mode(sound_mode).await?;
            println!("Sound mode set to {}", sound_mode);
            device.disconnect().await?;
        }

        Commands::Bass { level } => {
            let mut device = find_and_connect(&cli).await?;
            device.set_bass(level).await?;
            println!("Bass set to {}/20", level);
            device.disconnect().await?;
        }

        Commands::Treble { level } => {
            let mut device = find_and_connect(&cli).await?;
            device.set_treble(level).await?;
            println!("Treble set to {}/20", level);
            device.disconnect().await?;
        }

        Commands::QuietCouch { state } => {
            let mut device = find_and_connect(&cli).await?;
            device.set_quiet_couch(state.is_on()).await?;
            println!(
                "Quiet Couch Mode {}",
                if state.is_on() { "enabled" } else { "disabled" }
            );
            device.disconnect().await?;
        }

        Commands::Power { state } => {
            let mut device = find_and_connect(&cli).await?;
            device.set_power(state.is_on()).await?;
            println!("Power {}", if state.is_on() { "on" } else { "standby" });
            device.disconnect().await?;
        }

        Commands::Sniff { command } => {
            sniff::run(command, cli.scan_timeout).await?;
        }

        Commands::Serve { port } => {
            serve::run(port).await?;
        }

        Commands::Export { output } => {
            serve::export(&output)?;
        }
    }

    Ok(())
}

/// Scan for and connect to a StealthTech device.
///
/// When `--address` is provided, scans all BLE devices (not just StealthTech)
/// so devices with non-standard names can still be found by address.
async fn find_and_connect(cli: &Cli) -> anyhow::Result<StealthTechDevice> {
    let scanner = Scanner::new().await?;
    let timeout = Duration::from_secs(cli.scan_timeout);

    let device = if let Some(ref addr) = cli.address {
        let devices = scanner.scan_all(timeout).await?;
        devices
            .into_iter()
            .find(|d| d.address.to_lowercase() == addr.to_lowercase())
            .ok_or_else(|| anyhow::anyhow!("Device with address {} not found", addr))?
    } else {
        let devices = scanner.scan(timeout).await?;
        devices.into_iter().next().ok_or_else(|| {
            anyhow::anyhow!(
                "No StealthTech devices found. Make sure your center channel is powered on \
                     and nearby, or use --address to specify a device."
            )
        })?
    };

    println!(
        "Connecting to {} ({})...",
        device.name.as_deref().unwrap_or("Unknown"),
        device.address
    );

    StealthTechDevice::connect(device).await
}
