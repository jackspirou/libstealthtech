# Security Policy

## BLE Security Notice

The Lovesac StealthTech BLE control channel appears to use **no pairing or encryption**
for GATT operations. The official app connects as an unauthenticated BLE central, meaning
any device within BLE range (~30 feet / ~10 meters) can read and write GATT
characteristics. This is common for consumer audio devices but worth noting.

## What This Project Does

This project performs only **standard Bluetooth Low Energy operations**:

- Scanning for BLE advertisements
- Connecting to BLE peripherals
- Reading and writing GATT characteristics
- Subscribing to GATT notifications

These are the same operations performed by any BLE client, including the official
Lovesac StealthTech app.

## What This Project Does NOT Do

- Does not modify device firmware
- Does not bypass any DRM or encryption
- Does not access Lovesac servers or APIs
- Does not exploit any vulnerabilities
- Does not perform any operations beyond standard BLE GATT client behavior

## Reporting Security Issues

If you discover a security concern related to this project or the StealthTech BLE
protocol, please report it via
[GitHub Issues](https://github.com/jackspirou/libstealthtech/issues).

For issues related to the StealthTech hardware or official app, contact Lovesac directly.
