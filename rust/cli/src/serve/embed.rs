//! RustEmbed static file serving for the web UI.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use rust_embed::Embed;

/// Embedded static files from `src/serve/static/`.
///
/// The `debug-embed` feature is enabled, so files are compiled into the
/// binary in both debug and release builds. Recompile after changing static
/// files (including after `make wasm-embed`).
#[derive(Embed)]
#[folder = "src/serve/static/"]
pub struct StaticFiles;

/// Serve embedded static files, with SPA fallback to `index.html`.
///
/// - `.wasm` files are served with `application/wasm` MIME type
pub async fn static_handler(uri: axum::http::Uri) -> impl IntoResponse {
    let raw = uri.path().trim_start_matches('/');

    // Map directory-style URLs to their index.html
    let owned;
    let path = if raw.is_empty() {
        "index.html"
    } else if raw.ends_with('/') {
        owned = format!("{raw}index.html");
        &owned
    } else {
        raw
    };

    match StaticFiles::get(path) {
        Some(content) => {
            // Use explicit MIME for .wasm files; mime_guess may not know it
            let mime = if path.ends_with(".wasm") {
                "application/wasm".to_string()
            } else {
                mime_guess::from_path(path)
                    .first_or_octet_stream()
                    .to_string()
            };
            (
                [(axum::http::header::CONTENT_TYPE, mime)],
                content.data.into_owned(),
            )
                .into_response()
        }
        None => {
            // Don't SPA-fallback for asset requests — return 404 so the
            // browser gets a clear error instead of HTML with wrong MIME type.
            if path.contains('.') {
                return StatusCode::NOT_FOUND.into_response();
            }

            // SPA fallback to index.html (unified single-page UI)
            match StaticFiles::get("index.html") {
                Some(content) => (
                    [(
                        axum::http::header::CONTENT_TYPE,
                        "text/html".to_string(),
                    )],
                    content.data.into_owned(),
                )
                    .into_response(),
                None => StatusCode::NOT_FOUND.into_response(),
            }
        }
    }
}
