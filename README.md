# libstealthtech

> Open-source library and tools for controlling Lovesac StealthTech Sound + Charge systems via Bluetooth Low Energy.

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

The Lovesac StealthTech system is a Harman Kardon surround sound system embedded in
modular furniture, controlled via a BLE-connected mobile app that is notoriously
unreliable. This project provides a reliable, open-source alternative that works on
any platform with a Bluetooth adapter.

**The StealthTech BLE protocol has been fully reverse-engineered.** This project
can control volume, input, EQ, sound modes, power, and more over BLE.

## Repository Structure

```
libstealthtech/
├── rust/
│   ├── protocol/              # libstealthtech-protocol (WASM-compatible types)
│   ├── core/                  # libstealthtech-core library crate
│   │   └── src/
│   │       ├── ble/           # BLE scanning, connection, GATT discovery
│   │       └── device/        # High-level StealthTechDevice API
│   ├── cli/                   # CLI binary crate (stealthtech)
│   ├── bridge/                # FFI bridge for Swift/Java bindings (future)
│   └── wasm/                  # WebAssembly bindings for Web Bluetooth UI
├── swift/                     # iOS/macOS bindings (future)
├── java/                      # Android/Kotlin bindings (future)
├── docs/
│   ├── architecture.md        # Crate graph, BLE state machine, GATT flow
│   ├── protocol-mapping.md    # Complete BLE protocol specification
│   ├── reverse-engineering.md # RE methodology and tools
│   ├── firmware-analysis.md   # MCU firmware binary analysis
│   └── hardware-teardown.md   # Complete technical teardown
├── CONTRIBUTING.md
├── SECURITY.md
└── LICENSE
```

## Quick Start

```bash
# Clone and build
git clone https://github.com/jackspirou/libstealthtech.git
cd libstealthtech
cargo build --release

# Scan for StealthTech devices
./target/release/stealthtech scan

# Control your system
./target/release/stealthtech volume 18       # Set volume (0-36)
./target/release/stealthtech input hdmi      # Switch to HDMI ARC
./target/release/stealthtech mode movies     # Movies sound preset
./target/release/stealthtech bass 12         # Set bass (0-20)
./target/release/stealthtech power off       # Enter standby
```

### Reverse engineering tools

```bash
# Scan for ALL nearby BLE devices
./target/release/stealthtech sniff scan-all

# Dump the full GATT profile
./target/release/stealthtech sniff discover --json my_gatt_dump.json

# Monitor BLE notifications in real time
./target/release/stealthtech sniff monitor --log traffic.tsv
```

## Status

| Layer | Status |
|-------|--------|
| BLE Discovery & Connection | **Working** -- scanning, auto-reconnect with exponential backoff |
| GATT Service Enumeration | **Working** -- full profile dump and read-all tools |
| BLE Traffic Monitoring | **Working** -- real-time notification capture with logging |
| Protocol Command Encoding | **Complete** -- all commands fully mapped with real UUIDs |
| Volume / Input / Mode Control | **Working** -- volume, bass, treble, input, sound mode, mute |
| EQ / Balance / Power | **Working** -- center, rear, balance, quiet mode, power on/off |
| Notification Decoding | **Working** -- all 15 response codes parsed from UpStream |
| Firmware Version Query | **Working** -- MCU, DSP, EQ version retrieval |

## Hardware Background

StealthTech is built from these vendor components:

| Component | Vendor | Key Detail |
|-----------|--------|------------|
| Center Channel (EE4034) | Harman International | 242W RMS, BLE + HDMI ARC |
| Subwoofer (EE0362) | Harman International | 50W RMS, 8" driver, WiSA receiver |
| Sound+Charge Sides | Harman International | 52W RMS each, 6.5" + tweeter |
| WiSA Module (444-2250) | Summit Semiconductor | NXP MKW21D512 + Summit SWM908 ASIC, 5 GHz |
| Wireless Charging Pad | jjPlus Corp (Taiwan) | Qi standard, FCC ID 2A8R5QST008A |
| BLE Control | Harman International | BLE 4.2+, app control only |

The center channel acts as the BLE GATT server. The WiSA link (center to subwoofer to
wired speakers) is a separate 5 GHz radio -- we only control the center channel via BLE
to adjust volume, input, EQ, and other settings.

## How to Help

The protocol is mapped, but testing across hardware variants is valuable.
See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed instructions.

Ways to contribute:

1. **Test on your hardware** — run the CLI and report any issues
2. **Share GATT dumps** — `stealthtech sniff discover --json gatt_dump.json`
3. **Report device names** — different firmware versions may advertise differently
4. **Code contributions** — Swift/Android bindings, GUI app, additional features

## Legal

This project performs only standard Bluetooth Low Energy operations (scanning, connecting,
reading/writing GATT characteristics) on hardware you own. This is functionally identical
to what the official app does, just from a different client.

The project does not:

- Bypass any DRM or encryption
- Modify device firmware
- Access any Lovesac servers or APIs
- Redistribute any Lovesac or Harman Kardon intellectual property

## License

MIT License. See [LICENSE](LICENSE).
