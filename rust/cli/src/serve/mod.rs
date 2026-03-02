//! Web server for browser-based StealthTech remote control.
//!
//! Provides a REST API for device control, a WebSocket endpoint for
//! real-time BLE notification streaming, and an embedded SPA web UI.

mod api;
mod embed;
mod state;
mod ws;

use std::path::Path;

use axum::Router;
use tower_http::cors::CorsLayer;

/// Start the web server on the given port.
///
/// This is the entry point for the `stealthtech serve` subcommand.
pub async fn run(port: u16) -> anyhow::Result<()> {
    let state = state::AppState::new().await?;

    let app = Router::new()
        .nest("/api", api::routes())
        .route("/ws", axum::routing::get(ws::handler))
        .fallback(embed::static_handler)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("StealthTech web UI: http://localhost:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}

/// Export static Web Bluetooth files to a directory.
///
/// Writes the embedded UI files needed for standalone Web Bluetooth operation
/// (no server required). The output can be opened from `file://` in Chrome
/// or hosted on any static file server.
pub fn export(output: &Path) -> anyhow::Result<()> {
    // Files needed for standalone Web Bluetooth mode
    let files = [
        "style.css",
        "shared.js",
        "bluetooth.js",
        "favicon.svg",
        "manifest.json",
        "robots.txt",
        "sitemap.xml",
        "pkg/libstealthtech_wasm.js",
        "pkg/libstealthtech_wasm_bg.wasm",
    ];

    std::fs::create_dir_all(output)?;

    // Write the standalone bluetooth-only index.html
    let index_html = embed::StaticFiles::get("standalone.html")
        .ok_or_else(|| anyhow::anyhow!("standalone.html not found in embedded files"))?;
    std::fs::write(output.join("index.html"), index_html.data.as_ref())?;
    println!("  index.html");

    for file in &files {
        let path = Path::new(file);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(output.join(parent))?;
            }
        }

        match embed::StaticFiles::get(file) {
            Some(content) => {
                std::fs::write(output.join(file), content.data.as_ref())?;
                println!("  {}", file);
            }
            None => {
                eprintln!(
                    "  warning: {} not found in embedded files (run `make wasm-embed` first)",
                    file
                );
            }
        }
    }

    println!("\nExported to: {}", output.display());
    println!("Open index.html in Chrome to use Web Bluetooth mode.");
    Ok(())
}
