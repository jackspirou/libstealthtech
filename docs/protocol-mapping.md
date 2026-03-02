# StealthTech Protocol Mapping

Complete BLE protocol specification for the Lovesac StealthTech Sound + Charge system.

Protocol reverse-engineered from MCU firmware analysis and confirmed by
[homebridge-lovesac-stealthtech](https://github.com/ohmantics/homebridge-lovesac-stealthtech)
(MIT, Alex Rosenberg).

## Custom GATT Service

The StealthTech service UUID encodes **"excelpoint.com"** in ASCII:

```
65786365-6c70-6f69-6e74-2e636f6d0000
 e x c e  l p  o i  n t  . c o m
```

[Excelpoint Technology](https://www.excelpoint.com) is a Singapore-based electronics
distributor that designed the BLE firmware for Harman Kardon.

All characteristics share this base UUID with the last 2 bytes varying.

## GATT Characteristics

| UUID Suffix | Name | Properties | Purpose |
|-------------|------|------------|---------|
| 0001 | UpStream | Notify | Device → host status notifications |
| 0002 | DeviceInfo | Write | Request state dump or firmware version |
| 0003 | EqControl | Write | Volume, bass, treble, center, rear, mute, quiet, preset |
| 0004 | AudioPath | Write | Balance, power |
| 0005 | PlayerControl | Write | Bluetooth media play/pause/skip |
| 0006 | SystemLayout | Write | Configuration shape |
| 0007 | Source | Write | Input source selection |
| 0008 | Covering | Write | Fabric type for acoustic tuning |
| 0009 | UserSetting | Write | User preferences |
| 000a | OTA | Write | Over-the-air firmware update |

Standard BLE services are also present: Generic Access (0x1800), Generic Attribute (0x1801),
Device Information (0x180A).

## Packet Formats

All writes use **WriteWithoutResponse**.

### Format A (5 bytes)

```
AA <cmd_id> <sub_cmd_id> 01 <value>
```

Used for EQ commands (cmd=0x03), audio path (cmd=0x04), player control (cmd=0x05).

### Format B (4 bytes)

```
AA <cmd_id> <value> 00
```

Used for preset (cmd=0x03), source (cmd=0x07), device info requests (cmd=0x01).

## Command Encoding Table

| Command | Characteristic | Format | cmd_id | sub_cmd_id | Value | Range |
|---------|---------------|--------|--------|------------|-------|-------|
| Volume | EqControl (0003) | A | 0x03 | 0x02 | level | 0-36 |
| Bass | EqControl (0003) | A | 0x03 | 0x01 | level | 0-20 |
| Treble | EqControl (0003) | A | 0x03 | 0x00 | level | 0-20 |
| Center Volume | EqControl (0003) | A | 0x03 | 0x03 | level | 0-30 |
| Rear Volume | EqControl (0003) | A | 0x03 | 0x0A | level | 0-30 |
| Mute | EqControl (0003) | A | 0x03 | 0x09 | 0/1 | bool |
| Quiet Mode | EqControl (0003) | A | 0x03 | 0x04 | 0/1 | bool |
| Preset | EqControl (0003) | B | 0x03 | — | see below | 5-9 |
| Balance | AudioPath (0004) | A | 0x04 | 0x00 | balance | 0-100 |
| Power | AudioPath (0004) | A | 0x04 | 0x01 | 0/1 | bool |
| Source | Source (0007) | B | 0x07 | — | see below | 0-3 |
| Play/Pause | PlayerControl (0005) | A | 0x05 | 0x00 | value | — |
| Skip | PlayerControl (0005) | A | 0x05 | 0x01 | value | — |
| Get State | DeviceInfo (0002) | B | 0x01 | — | 0x01 | — |
| Get Version | DeviceInfo (0002) | — | — | — | `AA 01 01 01` | — |

## Preset / Sound Mode Values

Write and read values differ:

| Mode | Write Value (to device) | Read Value (from notifications) |
|------|------------------------|---------------------------------|
| Movies | 7 | 0 |
| Music | 8 | 1 |
| TV | 5 | 2 |
| News | 6 | 3 |
| Manual | 9 | — (write-only) |

## Input Source Values

Same for read and write:

| Source | Value |
|--------|-------|
| HDMI-ARC | 0 |
| Bluetooth | 1 |
| AUX | 2 |
| Optical | 3 |

## Notification Protocol

Notifications arrive on the **UpStream** characteristic (0001).

### Status Notifications

Format: `CC 05/06 AA ... <response_code> <value>`

The last 2 bytes are always the response code and value.

| Code | Name | Value Range | Notes |
|------|------|-------------|-------|
| 0x01 | Volume | 0-36 | |
| 0x02 | Center Volume | 0-30 | |
| 0x03 | Treble | 0-20 | |
| 0x04 | Bass | 0-20 | |
| 0x05 | Mute | 0/1 | 1=muted |
| 0x06 | Quiet Mode | 0/1 | 1=enabled |
| 0x07 | Balance | 0-100 | 50=center |
| 0x08 | Layout | byte | Config shape |
| 0x09 | Source | 0-3 | See source table |
| 0x0A | Power | 0/1 | **INVERTED**: 0=ON, 1=OFF |
| 0x0B | Preset | 0-3 | See read values table |
| 0x0C | Covering | byte | Fabric type ID |
| 0x0D | Arm Type | byte | |
| 0x0E | Subwoofer | 0/1 | 1=connected |
| 0x0F | Rear Volume | 0-30 | |

### Version Notifications

Format: `CC 05/06 AA 01 03 <type> <major> <minor>`

| Type | Component |
|------|-----------|
| 0x01 | MCU |
| 0x02 | DSP |
| 0x03 | EQ |

Example: `CC 06 AA 01 03 01 01 47` = MCU version 1.71.

## Hardware Architecture

| Component | Chip | Communication |
|-----------|------|---------------|
| BLE/Audio SoC | Qualcomm QCC3008 | UART (AT commands) to MCU |
| MCU | ARM Cortex-M | I2C to DSP/amp, UART to QCC3008 |
| DSP | Analog Devices ADAU1452 SigmaDSP | I2C from MCU |
| Amplifier | Texas Instruments TAS5825M | I2C from MCU |
| HDMI/SPDIF | SP8107 | I2C/SPI from MCU |
| WiSA TX | Summit Semiconductor | SWM API from MCU |

## Known Firmware Versions

| Package | APP | MCU | EQ | Date |
|---------|-----|-----|-----|------|
| V-1.62 | Unknown | Unknown | Unknown | Unknown |
| V-1.66 | V-1.66 | V-1.66 | V-1.23 | 2023-05-24 |
| V-1.71 | V-1.68 | V-3.71 | V-1.23 | 2024 |

BT chipset firmware: QCC3008 v3.0 (2023-09-18).

## References

- [homebridge-lovesac-stealthtech](https://github.com/ohmantics/homebridge-lovesac-stealthtech) — MIT reference implementation
- [Firmware Analysis](firmware-analysis.md) — MCU binary string analysis
- [Reverse Engineering Guide](reverse-engineering.md) — methodology and tools
- `rust/protocol/src/` — Rust implementation
