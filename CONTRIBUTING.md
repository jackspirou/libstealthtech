# Contributing to libstealthtech

The most important contribution right now is **reverse engineering the BLE protocol**.
You don't need to be a Rust developer to help -- you just need a StealthTech system
and a Bluetooth-capable computer or phone.

## Priority #1: GATT Profile Discovery

Run the discovery tool and share the output:

```bash
cargo build --release -p stealthtech-tools
./target/release/stealthtech sniff discover --json my_gatt_dump.json
```

This will connect to your StealthTech center channel and dump every BLE service and
characteristic it exposes. The JSON file will contain the UUIDs, properties, and
current values of all characteristics.

**Submit the JSON file as a GitHub issue** titled "GATT Profile Dump -- [your firmware version]".

Include:
- Your firmware version (visible on the center channel's LED display during boot, or in the app)
- Your center channel model number (EE4034, GA4364, etc.)
- Whether your system is pre- or post-January 2025 Bluetooth chipset update

## Priority #2: BLE Traffic Capture

This requires simultaneously running the official app and our sniffer:

### Method A: Using stealthtech sniff (requires 2 BLE adapters or 2 devices)

Since BLE only allows one connection at a time, you need to sniff at the advertising
level or use the Android method below. However, our monitor command captures
notifications after connection:

```bash
./target/release/stealthtech sniff monitor --log traffic.tsv
```

Then use the **physical remote** (which communicates via a separate BLE/RF channel)
to change volume, input, mode, etc. The monitor will capture any notifications the
center channel sends.

### Method B: Android HCI Snoop Log (RECOMMENDED)

This is the most powerful approach -- it captures ALL Bluetooth traffic at the HCI level.

1. **Enable Developer Options** on your Android phone:
   - Settings -> About Phone -> Tap "Build Number" 7 times

2. **Enable Bluetooth HCI Snoop Log**:
   - Settings -> Developer Options -> Enable Bluetooth HCI Snoop Log
   - (On some phones: Settings -> Developer Options -> Networking -> Bluetooth HCI Snoop Log)

3. **Open the official Lovesac StealthTech app** and connect to your system

4. **Perform specific actions one at a time**, with pauses between:
   - Change volume from 20 to 25
   - Switch input to HDMI ARC
   - Switch input to Bluetooth
   - Change sound mode to Movie
   - Change sound mode to Music
   - Adjust bass up
   - Adjust treble down
   - Save a profile
   - Load a profile

5. **Retrieve the snoop log**:
   ```bash
   adb pull /data/misc/bluetooth/logs/btsnoop_hci.log
   # or on newer Android:
   adb bugreport bugreport.zip
   # The log is inside the zip at FS/data/misc/bluetooth/logs/
   ```

6. **Analyze with Wireshark**:
   - Open the `.log` file in Wireshark
   - Filter: `btatt` (to see GATT operations)
   - Look for Write Request and Write Command packets
   - Note the characteristic UUID and the data being written

7. **Submit your findings** as a GitHub issue with:
   - The action you performed
   - The characteristic UUID that was written to
   - The hex bytes that were written
   - Any notifications received in response

### Method C: iOS PacketLogger

1. Install **Additional Tools for Xcode** from Apple Developer
2. Use the **PacketLogger** tool to capture Bluetooth traffic
3. Same procedure as Android -- perform actions one at a time

## Priority #3: Protocol Documentation

Once we have enough traffic captures, we need to:

1. Map each GATT characteristic UUID to its function
2. Decode the byte format for each command (endianness, ranges, etc.)
3. Document any command sequences (e.g., does changing input require multiple writes?)
4. Identify notification patterns (what does the device send back?)

Add your findings to `rust/protocol/src/characteristics.rs` with documentation of
how the mapping was discovered, and update `docs/protocol-mapping.md` with a new entry in
the Confirmed Findings Log.

## Code Contributions

For Rust code contributions:

1. Fork the repo
2. Create a feature branch
3. Run `cargo fmt` and `cargo clippy` before committing
4. Add tests where possible
5. Submit a PR with a clear description

### Architecture guidelines

- **`rust/protocol/src/`** -- Protocol encoding/decoding, GATT UUIDs, WASM-compatible (no BLE dependency)
- **`rust/core/src/ble/`** -- Pure BLE operations, no StealthTech protocol knowledge
- **`rust/core/src/device/`** -- High-level API combining BLE and protocol layers
- **`rust/cli/src/`** -- User-facing CLI tool (`stealthtech` binary with `sniff` subcommand)

## What NOT to submit

- Decompiled app source code (IP concerns)
- Firmware binary dumps
- Anything obtained by modifying the device hardware
- Credentials or personal data from BLE captures

We only want protocol findings obtained through standard BLE observation (the same
type of communication any BLE device does in the open).
