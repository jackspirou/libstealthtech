.PHONY: all build release check fmt fmt-check clippy test doc clean install ci sniff-scan sniff-discover sniff-monitor serve serve-dev bridge wasm wasm-embed release-all clean-wasm protocol

# Default target
all: check

# Debug build (all workspace members)
build:
	cargo build --workspace

# Release build (binaries only)
release:
	cargo build --release -p stealthtech-tools

# Check compilation without producing artifacts
check:
	cargo check --workspace

# Format all Rust code
fmt:
	cargo fmt --all

# Check formatting without modifying (used in CI)
fmt-check:
	cargo fmt --all -- --check

# Run clippy lints
clippy:
	cargo clippy --workspace --all-targets -- -D warnings

# Run tests
test:
	cargo test --workspace

# Generate and open documentation
doc:
	cargo doc --workspace --no-deps --open

# Remove build artifacts
clean:
	cargo clean

# Install the CLI tools to ~/.cargo/bin
install:
	cargo install --path rust/cli --bins

# Combined CI check (runs locally what CI runs remotely)
ci: fmt-check clippy test build

# Reverse engineering tool shortcuts
sniff-scan:
	cargo run --release -p stealthtech-tools --bin stealthtech -- sniff scan-all

sniff-discover:
	cargo run --release -p stealthtech-tools --bin stealthtech -- sniff discover

sniff-monitor:
	cargo run --release -p stealthtech-tools --bin stealthtech -- sniff monitor

# Web server
serve:
	cargo run --release -p stealthtech-tools --bin stealthtech -- serve

serve-dev:
	RUST_LOG=debug cargo run -p stealthtech-tools --bin stealthtech -- serve

# Native bindings (uniffi)
bridge:
	cargo build -p libstealthtech-bridge

# WASM
wasm:
	wasm-pack build rust/wasm --target web --out-dir ../../pkg/

# WASM -- copy pkg into static dir for embedding
wasm-embed: wasm
	mkdir -p rust/cli/src/serve/static/pkg
	cp pkg/libstealthtech_wasm_bg.wasm rust/cli/src/serve/static/pkg/
	cp pkg/libstealthtech_wasm.js rust/cli/src/serve/static/pkg/

# Release build with embedded WASM
release-all: wasm-embed release

# Clean WASM artifacts
clean-wasm:
	rm -rf rust/cli/src/serve/static/pkg
	rm -rf pkg/

# Protocol crate only
protocol:
	cargo check -p libstealthtech-protocol
