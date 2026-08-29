//! Static asset serving (rust-embed) with SPA fallback.

use axum::body::Body;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::Response;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

pub async fn serve_static(uri: &str) -> Response {
    let path = uri.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    if let Some(file) = Assets::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, HeaderValue::from_str(mime.as_ref()).unwrap_or(HeaderValue::from_static("application/octet-stream")))
            .body(Body::from(file.data.into_owned()))
            .unwrap_or_else(|_| Response::new(Body::from("internal error")));
    }

    // SPA fallback: unknown non-API paths serve index.html.
    if !path.starts_with("api/") {
        if let Some(index) = Assets::get("index.html") {
            return Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, HeaderValue::from_static("text/html"))
                .body(Body::from(index.data.into_owned()))
                .unwrap_or_else(|_| Response::new(Body::from("not found")));
        }
    }

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("not found"))
        .unwrap()
}

