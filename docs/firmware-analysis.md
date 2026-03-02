# StealthTech Firmware Analysis

Reverse engineering findings from analyzing Lovesac's official firmware downloads,
available at https://www.lovesac.com/stealthtech-firmware-updates. Two firmware
packages were examined: the main system firmware (V1.71) and the Bluetooth chipset
update tool (V3.0.1 for both Windows and macOS).

## Source Files Analyzed

### 1. Lovesac_Firmware_V1.71.zip (1.5 MB)

The main system firmware package contains three binary images and a configuration file.

| File | Size (bytes) | SHA256 (prefix) | Description |
|------|-------------|-----------------|-------------|
| `app.bin` | 1,727,488 | `6380c6f6...` | Main application firmware for the DSP/audio processor |
| `eq.bin` | 143,360 | `0938a9c6...` | Equalizer coefficient tables for the ADAU1452 SigmaDSP |
| `mcu.bin` | 60,636 | `9836e8ec...` | ARM Cortex-M microcontroller firmware |
| `LS_config.txt` | 32 | -- | Version manifest: `APP:V-1.68;MCU:V-3.71;EQ:V-1.23;` |

The MCU binary contains a standard ARM Cortex-M vector table at offset 0x00000000
with initial stack pointer `0x200037d8` and reset vector `0x000020c1` (bit 0 set,
confirming Thumb-2 instruction set).

The `LS_config.txt` version manifest uses semicolon-delimited key-value pairs. The
MCU reads this file during USB firmware update to determine which components to
flash. Note the version mismatch: the zip is labeled V1.71 but the config says
`APP:V-1.68` -- this suggests the zip was updated with a newer MCU binary while the
app binary was retained from an earlier build.

### 2. Lovesac_BT_Update_3.0.1_(Windows).zip (5.8 MB)

Windows Bluetooth chipset update tool. Contains `LovesacAISetup.msi` which installs
`BTFirmware.exe`, built with Advanced Installer 21.3.1.

**Bundled libraries:**

| Library | Purpose |
|---------|---------|
| `QTIL.HostTools.Common.Util.dll` | Qualcomm Technologies International Ltd host utility |
| `QTIL.HostTools.Common.Transport.dll` | Qualcomm transport layer |
| `QTIL.HostTools.Common.Dialogs.dll` | Qualcomm UI dialogs |
| `HidDfu.dll` | HID Device Firmware Update protocol handler |
| `HID.DLL` | USB Human Interface Device communication |
| `EngineFrameworkClr.dll` | .NET CLR engine wrapper |
| `EngineFrameworkCpp.dll` | Native C++ engine framework |
| `SETUPAPI.dll` | Windows USB device enumeration |

**Firmware images:** `file_3_0.bin` and `file_2_8.bin` -- likely QCC3008 firmware
images (version 3.0 and an older 2.8 fallback).

### 3. Lovesac_BT_Update_3.0.1(0.3).zip (1.1 MB)

macOS Bluetooth chipset update tool. Native macOS application, Universal binary
(x86_64 + arm64).

| Property | Value |
|----------|-------|
| Bundle ID | `com.citrusbits.NewTestingApp` |
| Developer | CitrusBits (agency) |
| Xcode version | 15.0.1 |
| macOS SDK | 14.0 |
| USB framework | USBDeviceSwift v1.0.3 (CocoaPods) |

The binary contains the string `qcc3008_sofa_3.0_20230918`, which is the definitive
confirmation of the Bluetooth chipset identity, variant name, firmware version, and
build date.

---

## Critical Finding 1: Bluetooth Chipset Identification

The BLE chipset is a **Qualcomm QCC3008**. This is confirmed by four independent
lines of evidence:

1. The string `qcc3008_sofa_3.0_20230918` embedded in the macOS BT update binary
2. QTIL (Qualcomm Technologies International Ltd) host tools DLLs in the Windows
   installer
3. The HID DFU (Device Firmware Update) protocol used -- this is the standard
   Qualcomm upgrade protocol for the QCC30xx series
4. The upgrade protocol message classes match the Qualcomm QCC DFU specification

The QCC3008 is a Qualcomm Bluetooth 5.0 audio SoC commonly used in headphones and
speakers. Its capabilities include:

- Bluetooth Low Energy (BLE) for control
- Classic Bluetooth A2DP/HFP for audio streaming
- Qualcomm aptX codec support
- On-chip DSP for audio processing

The "sofa" variant designation in the firmware string suggests Qualcomm or Harman
maintains a product-specific firmware branch for the Lovesac integration.

---

## Critical Finding 2: Hardware Architecture

The MCU firmware strings reveal the complete silicon bill of materials for the center
channel.

### Chip Identification

| Component | Chip | Role |
|-----------|------|------|
| BLE/Audio SoC | Qualcomm QCC3008 | BLE GATT server, Classic BT A2DP, audio decoding |
| MCU | ARM Cortex-M (vendor TBD) | System controller, I2C master, UART bridge to QCC3008 |
| DSP | Analog Devices ADAU1452 SigmaDSP | Audio processing, EQ, crossovers, delay, mixing |
| Amplifier | Texas Instruments TAS5825M | Class-D audio amplifier with I2C control |
| HDMI/SPDIF | SP8107 | HDMI ARC and optical SPDIF audio extraction |
| WiSA TX | Summit Semiconductor SWM series | 5 GHz wireless speaker network |

**Evidence:**
- `Dsp1452` references in MCU firmware strings identify the ADAU1452.
- Multiple `TAS5825M` I2C read/write debug strings confirm the amplifier IC.
- `SP8107` references confirm the HDMI/SPDIF extractor.
- Summit SWM API calls (`SWM_Network_SetMute()`, `SWM_Network_SetVolume`) confirm
  the WiSA transmitter.
- The MCU firmware contains the string `MCU FireWare Version V-1.71` (note the
  "FireWare" typo -- this is exactly as it appears in the binary).

### Inter-Chip Communication

The MCU serves as the central controller, bridging all subsystems:

```
                     UART (AT commands)
    QCC3008 BLE  <──────────────────────>  ARM Cortex-M MCU
                                              │
                                    ┌─────────┼─────────┐
                                    │         │         │
                                  I2C       I2C      SWM API
                                    │         │         │
                                    ▼         ▼         ▼
                               ADAU1452   TAS5825M   Summit
                               SigmaDSP  Class-D    WiSA TX
                                          Amp
```

The MCU receives BLE data from the QCC3008 over UART, translates it into I2C
commands for the DSP and amplifier, and SWM API calls for WiSA speaker control.

UART buffer variables visible in the firmware: `Rx2Buf`, `Tx2Buf` with
`CommandDone` and `CommandCnt` fields, indicating a command-response protocol
between the MCU and BLE chip.

---

## Critical Finding 3: AT Command Protocol (MCU to QCC3008)

The MCU firmware reveals the AT command set used for UART communication with the
QCC3008. These commands are sent FROM the MCU TO the BLE chip to control Bluetooth
behavior.

| Command | Function |
|---------|----------|
| `AT+ON` | Power on BLE radio |
| `AT+OFF` | Power off BLE radio |
| `AT+ADV=1` | Enable BLE advertising (make device discoverable) |
| `AT+PAIR=180` | Enable Bluetooth pairing for 180 seconds |
| `AT+PAIR=0` | Disable Bluetooth pairing |
| `AT+PLAY` | Play audio (A2DP stream) |
| `AT+PAUSE` | Pause audio |
| `AT+FWD` | Skip forward (AVRCP) |
| `AT+BWD` | Skip backward (AVRCP) |
| `AT+RESET` | Reset Bluetooth module |
| `AT+RC?` | Query remote control connection status |
| `AT+RCE` | Remote control event |
| `AT+CDL` | Clear device list (unpair all devices) |
| `AT+PDL` | Print/query paired device list |
| `AT+DEVC` | Connect to a Bluetooth device |
| `AT+DEVD` | Disconnect Bluetooth device |
| `AT+DFLT` | Reset Bluetooth module to factory defaults |
| `AT+DFU` | Enter Device Firmware Update mode |
| `AT+GATTD` | GATT data exchange (BLE data bridge) |
| `AT+VER` | Query Bluetooth firmware version |
| `AT+AUDIO` | Audio control/routing command |

The `AT+GATTD` command is the critical bridge between BLE and the MCU. When the
phone app writes to a GATT characteristic, the QCC3008 forwards the data to the MCU
via `AT+GATTD`, and the MCU processes it and executes the corresponding action (e.g.,
changing volume via I2C to the DSP, switching input source, etc.).

---

## Critical Finding 4: System State Model

The MCU firmware debug strings expose the complete internal state model. All values
are single bytes, formatted as `%02X` (two-digit hexadecimal).

### Global System State (`gSys.` struct)

| Variable | Description | Format |
|----------|-------------|--------|
| `gSys.VolLevel` | Current volume level | `%02X` |
| `gSys.Source_State` | Current input source | `%02X` |
| `gSys.Power_status` | Power state | `%02X` |
| `gSys.MuteState` | Mute on/off | `%02X` |
| `gSys.SystemMode` | Current sound mode | `%02X` |
| `gSys.Wisa_Status` | WiSA connection status (e.g., `WISA_CONNECTED`) | string |
| `gSys.SpeakCount` | Number of connected WiSA speakers | integer |
| `gSys.DspPowerState` | DSP chip power state | -- |
| `gSys.Audio_status` | Audio pipeline status | -- |
| `gSys.Pair_status` | Bluetooth pairing status | -- |
| `gSys.BtDevConectState` | BT device connection state (0 or 1) | integer |
| `gSys.AlreadyPairFlag` | Whether a device is already paired | boolean |
| `gSys.LostCnt` | Connection loss counter | integer |
| `gSys.Wisa_Pair_Times` | WiSA pairing attempt counter | integer |
| `gSys.BtPairLedCnt` | BT pairing LED blink counter | integer |
| `gSys.CdListCntTemp` | Connected device list count (max 8) | integer |
| `gSys.PdListCntTemp` | Paired device list count (max 8) | integer |

### Application Data (`AppDate.` struct)

Note: the struct is named `AppDate` in the firmware, not `AppData`. This typo is
present throughout the binary and is documented here exactly as it appears.

| Variable | Description | Format |
|----------|-------------|--------|
| `AppDate.Center_vol` | Center channel volume | `%02X` |
| `AppDate.TrebleVal` | Treble EQ value | `%02X` |
| `AppDate.BassVal` | Bass EQ value | `%02X` |
| `AppDate.QuiteMode` | Quiet Couch mode (note typo: "Quite" not "Quiet") | `%02X` |
| `AppDate.BlanceVal` | Balance value (note typo: "Blance" not "Balance") | `%02X` |
| `AppDate.CovingVal` | Covering/fabric type profile | `%02X` |
| `AppDate.ArmType` | Arm type configuration | `%02X` |
| `AppDate.RearChannelVol` | Rear channel volume | `%02X` |
| `AppDate.SystemLayoutVal` | System layout/configuration shape | `%02X` |
| `AppDate.DspVer` | DSP firmware version | `%02X` |
| `AppDate.TuningVer` | Tuning/EQ version | `%02X` |

### Debug Log Format Strings

These three debug format strings reveal the complete state snapshot that the MCU logs
on every state change. They confirm that every setting is communicated as a single
byte.

```
gSys.VolLevel=%02X  AppDate.Center_vol=%02X  AppDate.TrebleVal=%02X  AppDate.BassVal=%02X  gSys.MuteState=%02X  AppDate.QuiteMode=%02X  AppDate.BlanceVal=%02X
```

```
AppDate.CovingVal=%02X AppDate.ArmType=%02X AppDate.RearChannelVol:%02X
```

```
AppDate.SystemLayoutVal=%02X  gSys.Source_State=%02X  gSys.Power_status=%02X  AppDate.DspVer=%02X  AppDate.TuningVer=%02X  gSys.SystemMode=%02X
```

---

## Critical Finding 5: Input Sources

The MCU firmware defines four input sources with corresponding debug strings for
source changes.

| Internal Name | Description | Debug String |
|---------------|-------------|-------------|
| `SOURCE_HDMI` | HDMI ARC input | `Source_Change to HDMI` |
| `SOURCE_OPTICAL` | Optical/TOSLINK SPDIF | `Source_Change to OPTICAL` |
| `SOURCE_BLUETOOTH` | Bluetooth A2DP audio | `Source_Change to BT` |
| `SOURCE_AUX` | 3.5mm auxiliary input | `Source_Change to AUX` |

An additional debug string `Source_Change to BT RC` suggests a separate "Bluetooth
Remote Control" source state, possibly for when the remote control itself is
streaming audio or for distinguishing between phone Bluetooth and remote Bluetooth
connections.

---

## Critical Finding 6: Sound Modes

Four sound modes are defined, each with a corresponding debug string.

| Internal Name | Mode | Debug String |
|---------------|------|-------------|
| Movie | Movie surround mode | `MODE_Change to MOVIE` |
| Music | Music stereo/surround mode | `MODE_Change to MUSIC` |
| TV | TV dialog mode | `MODE_Change to TV` |
| Voice | Voice/News clarity mode | `MODE_Change to VOICE` |

Quiet Couch mode is stored separately in `AppDate.QuiteMode`, not as a sound mode
enum value. This suggests it is an independent toggle that can be combined with any
of the four sound modes.

---

## Critical Finding 7: Internal Message/Key System

The MCU uses a message queue system with key codes for internal event routing. These
key codes represent both physical button presses (from the remote via BLE) and
software-generated events (from the app via GATT).

### Key Codes

| Key Code | Function |
|----------|----------|
| `IN_KEY_POWER` | Power button |
| `IN_KEY_VOLSET` | Volume set command |
| `IN_KEY_SOURCE` | Source/input change |
| `IN_KEY_SOURCE_BT` | Switch to Bluetooth source |
| `IN_KEY_PAIR` | Enter Bluetooth pairing mode |
| `IN_KEY_MULTI_PAIR` | Multi-device pairing |
| `IN_KEY_RECONNECT` | Reconnect to last device |
| `IN_KEY_RESTART` | System restart |
| `IN_KEY_WISAREST` | WiSA reset/re-pair |

### Volume and EQ Control Strings

| String | Function |
|--------|----------|
| `VOL+` / `VOL-` | Volume increment/decrement |
| `VOL Set%d` | Set volume to absolute value |
| `BT_SetVol(%d)` | Set Bluetooth playback volume |
| `Sp8107 Vol Value` | HDMI/SPDIF input volume |
| `BASS+` / `BASS-` | Bass increment/decrement |
| `ROOM BALANCE-` | Room balance decrement |
| `Amp_Mute()` / `Amp_UnMute()` | Amplifier mute control |
| `CEC MUTE` / `CEC UNMUTE` | HDMI CEC mute control from TV remote |

The `CEC MUTE` / `CEC UNMUTE` strings confirm that the system supports HDMI CEC,
allowing volume and mute control from a connected TV's remote.

---

## Critical Finding 8: BLE Communication Protocol

### BLE Data Reception

The string `BlueTooth Ble Data:` indicates where the MCU receives raw BLE data
forwarded from the QCC3008 via the `AT+GATTD` UART command.

### Acknowledgment Protocol

The string `BT_Ble_Ack_SysInf acktype=(%d)  value=(%d)` reveals the BLE
acknowledgment structure:

1. The phone app writes to a GATT characteristic on the QCC3008
2. The QCC3008 forwards the data to the MCU via UART (`AT+GATTD`)
3. The MCU processes the command and executes the corresponding action
4. The MCU sends back an acknowledgment via `BT_Ble_Ack_SysInf` with a typed
   response (`acktype` identifies the setting, `value` contains the current state)
5. This acknowledgment is forwarded back through the QCC3008 to the phone app as a
   GATT notification

This confirms a structured command/response protocol where the `acktype` field
identifies which setting was changed and the `value` field contains the resulting
state.

### General BLE Handler

The string `Ble General ...........` suggests a catch-all handler for BLE data that
does not match any specific command parser. This may be where unknown or
vendor-specific GATT writes are logged.

### Remote Control vs App BLE

The MCU tracks BLE app and remote control connections separately:

| String | Meaning |
|--------|---------|
| `BlueTooth Rc is Connected` | Remote control connected via BLE |
| `BlueTooth Rc is DisConnected` | Remote control disconnected |
| `BLE Status   %d` | App BLE connection status |
| `RC Status   %d` | Remote control BLE connection status |

---

## Critical Finding 9: QCC3008 DFU Protocol

The Bluetooth update tools implement the **Qualcomm OTAU (Over-The-Air Upgrade)
Protocol** over USB HID. The complete protocol state machine was reconstructed from
the update tool binaries.

### Protocol State Machine

```
Host                              Device (QCC3008)
  |                                    |
  |--- UPGRADE_SYNC_REQ ------------->|
  |<-- UPGRADE_SYNC_CFM --------------|
  |                                    |
  |--- UPGRADE_START_REQ ------------>|
  |<-- UPGRADE_START_CFM -------------|
  |                                    |
  |--- UPGRADE_START_DATA_REQ ------->|
  |<-- UPGRADE_DATA_BYTES_REQ --------|  (device requests data chunks)
  |--- UPGRADE_DATA_REQ ------------>|  (host sends data chunk)
  |<-- UPGRADE_DATA_BYTES_REQ --------|  (device requests next chunk)
  |  ... (repeat until transfer complete) |
  |                                    |
  |<-- UPGRADE_TRANSFER_COMPLETE_IND -|
  |--- UPGRADE_TRANSFER_COMPLETE_RES >|
  |                                    |
  |--- UPGRADE_IS_VALIDATION_DONE_REQ>|
  |<-- UPGRADE_IS_VALIDATION_DONE_CFM-|
  |                                    |
  |--- UPGRADE_PROCEED_TO_COMMIT ---->|
  |--- UPGRADE_COMMIT_REQ ----------->|
  |<-- UPGRADE_COMMIT_CFM ------------|
  |                                    |
  |<-- UPGRADE_COMPLETE_IND ----------|
```

### Error and Abort Handling

```
  Error recovery:
  |<-- UPGRADE_ERROR_IND -------------|
  |--- UPGRADE_ERROR_RES ------------>|

  Transfer abort:
  |--- UPGRADE_ABORT_REQ ------------>|
  |<-- UPGRADE_ABORT_CFM -------------|

  Host version negotiation:
  |<-- UPGRADE_HOST_VERSION_REQ ------|
  |--- UPGRADE_HOST_VERSION_CFM ----->|
```

### Transport Layer

- **Windows**: USB HID with Feature Reports via `HID.DLL` and `SETUPAPI.dll`
- **macOS**: USB HID via `USBDeviceSwift` framework (CocoaPods, v1.0.3)

Key USB HID properties extracted from the macOS binary: `vendorID`, `productID`,
`inputReportByteLength`, `outputReportByteLength`, `featureReportByteLength`,
`MaxInputReportSize`, `MaxOutputReportSize`, `MaxFeatureReportSize`.

### Protocol Details

The protocol uses an `UpgradeProtocolOpCode` enum for message types and an
`UpgradeResumePoint` mechanism for recovering from interrupted transfers. CRC
validation is performed on the firmware image (`fileCRCIncorrect` error string
present in the binary).

### Physical Update Procedure

1. Connect the center channel to a PC via the Micro-USB "Service Port"
2. Put the device in upgrade mode using remote buttons (MODE + BT held for 12
   seconds, then Volume+ for 5 seconds)
3. The device enumerates as a USB HID device
4. The `BTFirmware.exe` (Windows) or macOS app pushes the QCC3008 firmware image
   via the USB HID DFU protocol described above

---

## Critical Finding 10: WiSA Speaker Configuration

The MCU firmware reveals the complete WiSA speaker position map, supporting up to
a 7.1 configuration.

### Speaker Positions

| Position Constant | Description |
|-------------------|-------------|
| `CENTER` | Center channel speaker |
| `Subwoofer` | Subwoofer |
| `Left Front` | Left front speaker |
| `Right Front` | Right front speaker |
| `LEFT_REAR` | Left rear surround |
| `RIGHT_REAR` | Right rear surround |
| `LEFT_SURROUND` | Left surround (side) |
| `RIGHT_SURROUND` | Right surround (side) |
| `CENTER_REAR` | Center rear (7.1 layout) |

### WiSA Pairing State Machine

The WiSA speaker pairing follows a defined state machine with two modes: classic
pairing and fast pairing.

**Classic pairing:**
```
SUMMIT_PAIR_START
  -> SUMMIT_PAIR_WAIT
    -> SUMMIT_PAIR_GET_SLAVE
      -> SUMMIT_PAIR_CLASSIC_STEP1
        -> SUMMIT_PAIR_CLASSIC_STEP2
          -> SUMMIT_PAIR_CLASSIC_END
            -> SUMMIT_PAIR_END
```

**Fast pairing:**
```
SUMMIT_PAIR_FAST
  -> SUMMIT_PAIR_FAST_END
```

The pairing timeout is 15 seconds, set via `TimeOutSet(&WisaPiarTimer,15000)` (note
the "Piar" typo in the timer variable name -- present in the firmware as-is).

---

## Critical Finding 11: EQ Binary Structure

The `eq.bin` file (143,360 bytes) contains DSP coefficient tables for the ADAU1452
SigmaDSP.

### Binary Layout

| Offset Range | Content |
|-------------|---------|
| `0x00` | Header -- register addresses and values (ADAU1452 parameter RAM format) |
| `0xA0` - `0x13F` | Frequency table -- ascending 32-bit values in fixed-point format |
| Various offsets | Multiple coefficient tables at different offsets, likely one per sound mode (Movie, Music, TV, Voice) |
| Large regions | `0xFF` fill (unused/erased flash regions) |

### Frequency Table Sample Values

The frequency table at offset `0xA0` contains ascending 32-bit values that appear to
be fixed-point frequency representations:

| Hex Value | Approximate Frequency |
|-----------|----------------------|
| `0x000000A8` | ~168 Hz |
| `0x000003AF` | ~943 Hz |
| ... | ascending |
| `0x01C73D52` | upper frequency limit |

These are likely center frequencies for parametric EQ bands or crossover filter
coefficients. The presence of multiple tables suggests separate EQ profiles for each
sound mode and possibly for different fabric types.

---

## Critical Finding 12: Remote Control

The Bluetooth remote (model EE3531) communicates via BLE and has the following button
layout:

### Button Map (from the official PDF)

```
Top row:     [POWER]              [?] [?]
D-pad:       [BASS+]
         [TREB-] [QUIET COUCH] [TREB+]
             [BASS-]
Bottom:  [MODE]              [MUTE]
         [VOL-]              [VOL+]
```

### Remote DFU Entry Sequence

To put the remote into firmware update mode:

1. Hold MODE + BT simultaneously for 12 seconds
2. Press and hold Volume+ for 5 seconds
3. The remote enters DFU mode and is discoverable as a USB HID device

---

## Implications for libstealthtech

### Confirmed Protocol Properties

1. **All settings are single bytes.** The debug format strings universally use
   `%02X` (single hex byte) for every setting value. Volume, bass, treble, source,
   mode, fabric, layout, balance, mute, and quiet couch mode are all single-byte
   values. This significantly simplifies the GATT characteristic encoding -- each
   writable characteristic likely accepts a single byte.

2. **The QCC3008 uses standard Qualcomm BLE GATT.** There is extensive documentation
   and open-source tooling for QCC30xx GATT services. The Qualcomm Audio Development
   Kit (ADK) documentation may list standard GATT service UUIDs used by this chip
   family.

3. **`AT+GATTD` is the bridge command.** BLE GATT data arrives at the QCC3008, which
   forwards it to the MCU via UART as `AT+GATTD` commands. The MCU processes the
   data, executes the action, and sends back acknowledgments via
   `BT_Ble_Ack_SysInf` with typed responses.

4. **The acknowledgment protocol is structured.** The `acktype` and `value`
   parameters in `BT_Ble_Ack_SysInf` indicate that the MCU responds with a type
   code identifying the setting and a value representing the current state. This
   maps directly to GATT notifications -- each `acktype` likely corresponds to a
   GATT characteristic UUID.

5. **HDMI CEC is supported.** The system responds to CEC volume and mute commands
   from a connected TV remote, as confirmed by the `CEC MUTE` and `CEC UNMUTE`
   debug strings.

### Expected GATT Service Layout

Based on the firmware analysis, the GATT profile should expose:

| Service | Expected UUID | Rationale |
|---------|--------------|-----------|
| Generic Access | `0x1800` | Standard BLE service |
| Device Information | `0x180A` | Firmware version, model, manufacturer strings |
| StealthTech Control | Custom 128-bit UUID (TBD) | Volume, source, mode, mute, power |
| StealthTech Tuning | Custom 128-bit UUID (TBD) | Bass, treble, balance, fabric, layout, quiet couch |

### Characteristic-to-Variable Mapping (Predicted)

Each GATT characteristic likely maps to one of the firmware state variables:

| Expected Characteristic | Firmware Variable | Byte Encoding (Predicted) |
|------------------------|-------------------|---------------------------|
| Volume | `gSys.VolLevel` | `0x00` - `0x3C` (0-60) or `0x00` - `0x64` (0-100) |
| Input Source | `gSys.Source_State` | Enum: HDMI, Optical, BT, AUX |
| Sound Mode | `gSys.SystemMode` | Enum: Movie, Music, TV, Voice |
| Mute | `gSys.MuteState` | `0x00` = unmuted, `0x01` = muted |
| Power | `gSys.Power_status` | `0x00` = off, `0x01` = on |
| Bass | `AppDate.BassVal` | Single byte, range TBD |
| Treble | `AppDate.TrebleVal` | Single byte, range TBD |
| Center Volume | `AppDate.Center_vol` | Single byte, range TBD |
| Rear Volume | `AppDate.RearChannelVol` | Single byte, range TBD |
| Balance | `AppDate.BlanceVal` | Single byte, center = `0x00` or `0x80` |
| Fabric Type | `AppDate.CovingVal` | Single byte enum/index (200+ fabrics) |
| Arm Type | `AppDate.ArmType` | Single byte enum |
| System Layout | `AppDate.SystemLayoutVal` | Enum: Straight, L-shape, U-shape, Pit |
| Quiet Couch | `AppDate.QuiteMode` | `0x00` = off, `0x01` = on |

### Next Steps for Protocol Discovery

1. **Search for QCC3008 GATT documentation.** Qualcomm publishes ADK (Audio
   Development Kit) documentation that may list standard GATT services used by
   QCC30xx chips. Cross-reference any published service UUIDs with the StealthTech
   device.

2. **Enumerate actual UUIDs.** Run `stealthtech sniff discover` against a live
   device to enumerate all GATT services and characteristics, then cross-reference
   with known QCC3008 GATT profiles.

3. **Monitor the MCU UART.** The USB service port on the center channel may expose
   the MCU UART. Connecting to it with a serial terminal could reveal the AT command
   traffic (including `AT+GATTD` data) and debug output in real-time, allowing
   direct observation of the BLE-to-MCU protocol.

4. **HCI snoop capture.** Enable Android HCI snoop logging while using the official
   Lovesac app, then analyze the captured BLE writes in Wireshark to map GATT
   characteristic UUIDs to specific functions.

5. **Correlate `acktype` values.** Once GATT writes are captured, the `acktype`
   values in the MCU acknowledgments should map 1:1 to GATT characteristic UUIDs
   or function codes, providing the complete protocol mapping.

---

## References

- [Reverse Engineering Guide](reverse-engineering.md) -- methodology and tools
- [Hardware Teardown](hardware-teardown.md) -- complete technical teardown
- [Architecture](architecture.md) -- crate structure and BLE state machine
- [Protocol Mapping](protocol-mapping.md) -- structured findings log for discovered UUIDs
- Lovesac firmware downloads: https://www.lovesac.com/stealthtech-firmware-updates
