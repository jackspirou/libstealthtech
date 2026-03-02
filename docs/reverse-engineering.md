# StealthTech Reverse Engineering Guide

## What We Know (as of 2026)

### Hardware Architecture

```
                    ┌────────────────────────┐
                    │    CENTER CHANNEL       │
                    │    Model: EE4034        │
                    │                         │
   HDMI ARC ───────┤  Harman Kardon DSP      │
   Optical ────────┤  Dolby Digital 5.1      │
   3.5mm AUX ─────┤  Dolby Pro Logic II     │
   Bluetooth A2DP ─┤                         │
                    │  ┌─────────┐ ┌───────┐ │
                    │  │ BLE 4.2 │ │ WiSA  │ │
                    │  │ (App    │ │ TX    │ │
                    │  │ Control)│ │ 5 GHz │ │
                    │  └────┬────┘ └───┬───┘ │
                    └───────┼──────────┼─────┘
                            │          │
                     BLE to phone    WiSA 5GHz
                     (this is what   (uncompressed
                      we control)     24-bit audio)
                                       │
                    ┌──────────────────┼─────┐
                    │   SUBWOOFER      │     │
                    │   Model: EE0362  │     │
                    │                  ▼     │
                    │   Summit WiSA RX      │
                    │   NXP MKW21D512       │
                    │   Summit SWM908       │
                    │                        │
                    │   8" Driver, 50W RMS   │
                    │                        │
                    │   6-pin ──► L Sound+Charge Side (GE2913, 52W)
                    │   6-pin ──► R Sound+Charge Side (GE0177, 52W)
                    │   3-pin ──► Satellite Sides (GE7065, 12W each)
                    └────────────────────────┘
```

### BLE Communication Model

The center channel is the **GATT Server** (peripheral).
Your phone/computer is the **GATT Client** (central).

Two separate Bluetooth connections exist:
1. **BLE (Low Energy)** — For app control (volume, input, EQ, profiles)
2. **Classic BT A2DP** — For audio streaming from phone

These are independent. You can control the system via BLE without any audio streaming.

### What the Official App Does Over BLE

Based on the app's open-source dependency list and behavior:

1. **Scans** for a device advertising as "StealthTech Sound + Charge" (or similar)
2. **Connects** via BLE using SwiftyBluetooth (iOS) or BLESSED (Android)
3. **Discovers GATT services** to enumerate available characteristics
4. **Reads** device info (firmware version, model number, etc.)
5. **Writes** control commands (volume, input, mode, EQ, fabric, config shape)
6. **Subscribes** to notifications for state changes (e.g., volume changed via remote)
7. Uses **HTTP** (Alamofire/Retrofit) to communicate with a Lovesac backend server
   — likely for telemetry, fabric database, and firmware version checking

### Known FCC IDs

| FCC ID | Grantee | Description |
|--------|---------|-------------|
| APILOVESAC | Harman International | Center channel BLE transmitter |
| APILOVESACR | Harman International | Center channel (revised, Sep 2024) |
| 2A8R5QST008A | The Lovesac Company | Qi wireless charging pad |
| UA9601 | Summit Semiconductor | WiSA module 444-2250 (in subwoofer) |

### Known Model Numbers

From the setup guide (V-56.1):
- **Center Channel**: EE4034 (original), GA4364 (revised)
- **Subwoofer Receiver**: EE0362, GA4956
- **L Sound+Charge Side**: GE2913, GA4408
- **R Sound+Charge Side**: GE0177, GR0692
- **Satellite Side**: GE7065, GR5482
- **Charging Pad**: QST008A, QST015C
- **Remote**: EE3531

### Known Firmware Versions

| Version | Release Date | Notes |
|---------|-------------|-------|
| V-1.62 | Unknown | Early version |
| V-1.66 MCU / V-1.66 APP / V-1.23 EQ | May 24, 2023 | Widely deployed |
| V-1.71 | 2024 | Latest known, via USB flash drive |

Firmware consists of 3 independently-versioned components:
- **MCU** — ARM Cortex-M microcontroller firmware (V-3.71, 60KB)
- **APP** — DSP/audio application logic (V-1.68, 1.7MB) — runs on ADAU1452 SigmaDSP
- **EQ** — DSP equalization coefficient tables (V-1.23, 143KB)

See [firmware-analysis.md](firmware-analysis.md) for complete binary analysis.

### What We Now Know (from firmware analysis)

**All settings are single bytes.** The MCU debug strings confirm `%02X` format for every
setting variable. The MCU internal state maps directly to BLE characteristics:

| MCU Variable | Function | Encoding |
|-------------|----------|----------|
| `gSys.VolLevel` | Volume | Single byte hex |
| `gSys.Source_State` | Input source (HDMI/Optical/BT/AUX) | Single byte enum |
| `gSys.SystemMode` | Sound mode (Movie/TV/Music/Voice) | Single byte enum |
| `AppDate.BassVal` | Bass EQ | Single byte hex |
| `AppDate.TrebleVal` | Treble EQ | Single byte hex |
| `AppDate.Center_vol` | Center channel volume | Single byte hex |
| `AppDate.BlanceVal` | L/R balance | Single byte hex |
| `AppDate.QuiteMode` | Quiet Couch mode | Single byte hex |
| `AppDate.CovingVal` | Fabric/covering type | Single byte hex |
| `AppDate.SystemLayoutVal` | Configuration shape | Single byte hex |
| `AppDate.ArmType` | Arm type | Single byte hex |

**The BLE chipset is a Qualcomm QCC3008.** This means the GATT profile follows
Qualcomm's audio ADK patterns. The MCU communicates with the QCC3008 via UART
AT commands (e.g., `AT+GATTD` for BLE data, `AT+ON`/`AT+OFF` for power).

### What We Still Need to Discover

1. **Device advertisement name** — What exactly does the center channel advertise as?
   Run `stealthtech sniff scan-all` to find out.

2. **Custom GATT service UUIDs** — The QCC3008 likely exposes 1-3 custom services
   beyond the standard Device Information Service (0x180A). These will have full 128-bit
   UUIDs. Run `stealthtech sniff discover` to enumerate them.

3. **Characteristic → MCU variable mapping** — Which GATT characteristic UUID corresponds
   to which MCU variable (e.g., which UUID writes `gSys.VolLevel`?). The primary method
   is Android HCI snoop logging while using the official app.

4. **Enum value tables** — What byte value = HDMI? What byte value = Movie mode?
   Firmware analysis confirms single-byte encoding but the specific enum values are not
   yet known.

5. **Notification patterns** — What does the device send back when state changes?
   The MCU sends `BT_Ble_Ack_SysInf acktype=(%d) value=(%d)` back through BLE.

## Reverse Engineering Methodology

### Step 1: GATT Enumeration

```bash
stealthtech sniff discover --json gatt.json
```

Expected output structure:
- Service 0x1800 (Generic Access) — standard
- Service 0x180A (Device Information) — has firmware version, model, manufacturer
- Service ????????-????-????-????-???????????? — **CUSTOM: likely main control**
- Service ????????-????-????-????-???????????? — **CUSTOM: possibly tuning/profiles**

### Step 2: Read All Values

```bash
stealthtech sniff read-all
```

Note down the hex values of all readable characteristics in the default/idle state.

### Step 3: Traffic Capture

Enable Android HCI snoop log, then in the official app:

**Test 1: Volume**
- Set volume to exactly 0, then 10, 25, 50, 75, 100
- Note the writes for each

**Test 2: Input**
- Switch through each input: HDMI → Optical → Bluetooth → AUX → HDMI
- Note the writes for each

**Test 3: Sound Mode**
- Switch through: Movie → TV → News → Music → Quiet Couch
- Note the writes

**Test 4: EQ**
- Set bass to min, mid, max
- Set treble to min, mid, max

### Step 4: Decode

Look at the captured writes. Based on firmware analysis, expect:
- **Single-byte values** for all settings (confirmed from MCU `%02X` format strings)
- Possible command prefix byte followed by parameter byte
- Match writes to the MCU variable table above

### Step 5: USB Service Port (Advanced)

The center channel has a Micro-USB "Service Port" on the rear panel. This port is used
for firmware updates but may also expose the MCU's UART debug output. Connecting a
serial terminal could reveal real-time AT command traffic between the QCC3008 and MCU,
including the exact BLE data being received.

### Tools

- **Wireshark** with Bluetooth dissectors
- **nRF Connect** (Android/iOS app) for interactive GATT exploration
- **gatttool** (Linux) for command-line GATT operations
- **btmon** (Linux) for kernel-level BLE tracing
- **stealthtech sniff** (this project) for StealthTech-specific tooling
- **Qualcomm QMDE** (Qualcomm Music Development Environment) -- if available, for QCC3008 analysis

## Security Considerations

The StealthTech BLE connection appears to use **no pairing or encryption** for the
GATT control channel — the official app connects as an unauthenticated BLE central.
This means anyone within BLE range (~30 feet) can control the system. This is common
for consumer audio devices but worth noting.

## References

- [Firmware Analysis](firmware-analysis.md) -- binary analysis of official firmware downloads
- [Protocol Mapping](protocol-mapping.md) -- structured findings log
- [Hardware Teardown](hardware-teardown.md) -- FCC filings and physical teardown
