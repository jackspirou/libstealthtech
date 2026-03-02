# Node.js Bindings (Future)

This directory is reserved for future Node.js/JavaScript bindings.

Two paths are available:

- **Protocol-only** — The WASM crate in `rust/wasm/` already provides protocol
  encoding/decoding via `wasm-bindgen`. Building with `--target nodejs` would make
  these bindings available to Node.js immediately. Apps would bring their own BLE
  library (e.g., `noble`).

- **Full BLE** — A dedicated crate using `napi-rs` to wrap `rust/core/` would provide
  native BLE scanning, connection, and device control directly from Node.js.

## Status

Not started. The protocol is fully mapped. See [CONTRIBUTING.md](../CONTRIBUTING.md) for how to help.
