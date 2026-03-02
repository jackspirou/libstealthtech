//! Sniff subcommand: BLE sniffer and GATT discovery for reverse engineering.

use std::time::Duration;

use clap::Subcommand;
use futures::StreamExt;

use libstealthtech_core::ble::connection::{Connection, ConnectionConfig};
use libstealthtech_core::ble::gatt;
use libstealthtech_core::ble::scanner::Scanner;
use libstealthtech_core::protocol::characteristics::STEALTHTECH_DEVICE_NAMES;

#[derive(Subcommand)]
pub enum SniffCommands {
    /// Scan for all nearby BLE devices (not just StealthTech).
    ScanAll,

    /// Discover and dump the complete GATT profile of a StealthTech device.
    Discover {
        /// Save the GATT profile as JSON.
        #[arg(long)]
        json: Option<String>,

        /// Device address to connect to (skips scan).
        #[arg(long)]
        address: Option<String>,
    },

    /// Monitor all BLE notifications from a StealthTech device.
    ///
    /// Use this while operating the physical remote or official app to
    /// capture the commands being sent to the center channel.
    Monitor {
        /// Device address to connect to (skips scan).
        #[arg(long)]
        address: Option<String>,

        /// Log output to file (TSV format).
        #[arg(long)]
        log: Option<String>,
    },

    /// Read all readable characteristics and display their values.
    ReadAll {
        /// Device address to connect to.
        #[arg(long)]
        address: Option<String>,
    },

    /// Write a raw hex value to a specific characteristic (advanced).
    WriteRaw {
        /// Device address.
        #[arg(long)]
        address: Option<String>,

        /// Characteristic UUID to write to.
        #[arg(long)]
        uuid: String,

        /// Hex-encoded data to write (e.g., "0a1b2c").
        #[arg(long)]
        data: String,
    },
}

pub async fn run(command: SniffCommands, scan_timeout: u64) -> anyhow::Result<()> {
    match command {
        SniffCommands::ScanAll => {
            let scanner = Scanner::new().await?;
            println!(
                "Scanning for ALL BLE devices for {} seconds...\n",
                scan_timeout
            );

            let devices = scanner
                .scan_all(Duration::from_secs(scan_timeout))
                .await?;

            println!("{:<30} {:<20} {:<8}", "NAME", "ADDRESS", "RSSI");
            println!("{}", "─".repeat(60));

            for device in &devices {
                let name = device.name.as_deref().unwrap_or("(unnamed)");
                let rssi = device
                    .rssi
                    .map(|r| format!("{} dBm", r))
                    .unwrap_or_else(|| "N/A".into());

                let lower = name.to_lowercase();
                let is_st = STEALTHTECH_DEVICE_NAMES
                    .iter()
                    .any(|pattern| lower.contains(pattern));
                let marker = if is_st { " <- StealthTech?" } else { "" };

                println!("{:<30} {:<20} {:<8}{}", name, device.address, rssi, marker);
            }

            println!("\nTotal: {} devices", devices.len());
        }

        SniffCommands::Discover { json, address } => {
            let (mut conn, name, addr) = connect_to_device(scan_timeout, address).await?;
            let profile = gatt::discover_gatt_profile(&mut conn, name, addr).await?;

            gatt::print_gatt_profile(&profile);

            if let Some(path) = json {
                let json_str = serde_json::to_string_pretty(&profile)?;
                std::fs::write(&path, &json_str)?;
                println!("\n✓ GATT profile saved to: {}", path);
                println!("  Please submit this file to the project!");
            }

            conn.disconnect().await?;
        }

        SniffCommands::Monitor { address, log } => {
            let (conn, _name, _addr) = connect_to_device(scan_timeout, address).await?;

            let chars = conn.characteristics();
            let notifiable: Vec<_> = chars
                .iter()
                .filter(|c| {
                    c.properties
                        .contains(btleplug::api::CharPropFlags::NOTIFY)
                        || c.properties
                            .contains(btleplug::api::CharPropFlags::INDICATE)
                })
                .cloned()
                .collect();

            println!(
                "Subscribing to {} notifiable characteristics...",
                notifiable.len()
            );
            for char in &notifiable {
                if let Err(e) = conn.subscribe(char).await {
                    eprintln!("Warning: failed to subscribe to {}: {}", char.uuid, e);
                }
            }

            let mut stream = conn.notifications().await?;

            let mut log_file = if let Some(ref path) = log {
                let file = std::fs::File::create(path)?;
                let mut writer = std::io::BufWriter::new(file);
                use std::io::Write;
                writeln!(writer, "TIMESTAMP\tUUID\tLEN\tHEX\tASCII")?;
                Some(writer)
            } else {
                None
            };

            println!("\n╔══════════════════════════════════════════════════════╗");
            println!("║  BLE NOTIFICATION MONITOR                           ║");
            println!("║  Now use the physical remote or official app to      ║");
            println!("║  trigger actions. All BLE traffic will be captured.  ║");
            println!("║  Press Ctrl+C to stop.                              ║");
            println!("╚══════════════════════════════════════════════════════╝\n");

            println!(
                "{:<12} {:<40} {:<6} HEX DATA",
                "TIME", "CHARACTERISTIC UUID", "LEN"
            );
            println!("{}", "─".repeat(90));

            while let Some(notification) = stream.next().await {
                let now = chrono::Local::now().format("%H:%M:%S%.3f");
                let uuid = notification.uuid.to_string();
                let hex_data = hex::encode(&notification.value);
                let ascii: String = notification
                    .value
                    .iter()
                    .map(|b| {
                        if b.is_ascii_graphic() || *b == b' ' {
                            *b as char
                        } else {
                            '.'
                        }
                    })
                    .collect();

                println!(
                    "{:<12} {:<40} {:<6} {} │ {}",
                    now,
                    uuid,
                    notification.value.len(),
                    hex_data,
                    ascii
                );

                if let Some(ref mut writer) = log_file {
                    use std::io::Write;
                    writeln!(
                        writer,
                        "{}\t{}\t{}\t{}\t{}",
                        now,
                        uuid,
                        notification.value.len(),
                        hex_data,
                        ascii
                    )?;
                }
            }
        }

        SniffCommands::ReadAll { address } => {
            let (mut conn, _name, _addr) = connect_to_device(scan_timeout, address).await?;

            let chars = conn.characteristics();
            let readable: Vec<_> = chars
                .iter()
                .filter(|c| c.properties.contains(btleplug::api::CharPropFlags::READ))
                .cloned()
                .collect();

            println!(
                "Reading {} readable characteristics...\n",
                readable.len()
            );

            println!(
                "{:<40} {:<16} {:<6} VALUE",
                "UUID", "SERVICE", "LEN"
            );
            println!("{}", "─".repeat(90));

            for char in &readable {
                match conn.read(char).await {
                    Ok(data) => {
                        let hex_str = hex::encode(&data);
                        let utf8 = String::from_utf8(data.clone())
                            .ok()
                            .filter(|s| {
                                s.chars()
                                    .all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace())
                            });

                        let display = if let Some(ref s) = utf8 {
                            format!("{} (\"{}\")", hex_str, s)
                        } else {
                            hex_str
                        };

                        println!(
                            "{:<40} {:<16} {:<6} {}",
                            char.uuid,
                            &char.service_uuid.to_string()[..8],
                            data.len(),
                            display
                        );
                    }
                    Err(e) => {
                        println!(
                            "{:<40} {:<16} {:<6} ERROR: {}",
                            char.uuid,
                            &char.service_uuid.to_string()[..8],
                            0,
                            e
                        );
                    }
                }
            }

            conn.disconnect().await?;
        }

        SniffCommands::WriteRaw {
            address,
            uuid,
            data,
        } => {
            let (mut conn, _name, _addr) = connect_to_device(scan_timeout, address).await?;

            let target_uuid: uuid::Uuid = uuid.parse()?;
            let raw_data = hex::decode(&data)?;

            let chars = conn.characteristics();
            let char = chars
                .iter()
                .find(|c| c.uuid == target_uuid)
                .ok_or_else(|| anyhow::anyhow!("Characteristic {} not found", uuid))?
                .clone();

            println!("Writing {} bytes to {}...", raw_data.len(), uuid);
            conn.write(
                &char,
                &raw_data,
                btleplug::api::WriteType::WithResponse,
            )
            .await?;
            println!("✓ Write successful");

            conn.disconnect().await?;
        }
    }

    Ok(())
}

/// Scan for and connect to a StealthTech device.
async fn connect_to_device(
    scan_timeout: u64,
    address: Option<String>,
) -> anyhow::Result<(Connection, Option<String>, String)> {
    let scanner = Scanner::new().await?;

    let device = if let Some(ref addr) = address {
        // When address is specified, scan all devices (not just StealthTech)
        let all = scanner
            .scan_all(Duration::from_secs(scan_timeout))
            .await?;
        all.into_iter()
            .find(|d| d.address.to_lowercase() == addr.to_lowercase())
            .ok_or_else(|| anyhow::anyhow!("Device {} not found", addr))?
    } else {
        let devices = scanner
            .scan(Duration::from_secs(scan_timeout))
            .await?;
        devices
            .into_iter()
            .next()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No StealthTech devices found. Use --address to specify a device, \
                     or run `stealthtech sniff scan-all` to see all BLE devices."
                )
            })?
    };

    println!(
        "Connecting to {} ({})...",
        device.name.as_deref().unwrap_or("Unknown"),
        device.address
    );

    let name = device.name.clone();
    let addr = device.address.clone();
    let mut conn = Connection::new(device.peripheral, ConnectionConfig::default());
    conn.connect().await?;

    println!("✓ Connected\n");
    Ok((conn, name, addr))
}
