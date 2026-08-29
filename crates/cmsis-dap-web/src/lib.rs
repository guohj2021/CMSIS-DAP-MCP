//! cmsis-dap-web: local Web Debug server (REST + WebSocket) over cmsis-dap-core.
//!
//! Entry point used by `cmsis-dap-cli web` (frozen v5 plan, P1-P4).

pub mod api;
pub mod assets;
pub mod executor;
pub mod flash;
pub mod monitor;
pub mod op;
pub mod schedulers;
pub mod resolver;
pub mod session;
pub mod watch;
pub mod state;
pub mod ws;

use crate::executor::{spawn, ExecutorConfig};
use crate::op::{OperationKind, WebError};
use crate::session::SessionLease;
use crate::state::{AppState, SharedState};
use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use cmsis_dap_core::backend::{Backend, ConnectOptions};
use cmsis_dap_core::session::SessionManager;
use std::sync::Arc;
use std::time::Duration;

/// Options for the web server.
#[derive(Debug, Clone)]
pub struct WebServerOptions {
    pub host: String,
    pub port: u16,
    pub allow_destructive: bool,
    pub flash_timeout: Duration,
    /// Defaults used when the UI omits connect fields.
    pub default_connect: ConnectOptions,
}

impl Default for WebServerOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8080,
            allow_destructive: false,
            flash_timeout: Duration::from_secs(600),
            default_connect: ConnectOptions {
                probe_id: None,
                protocol: cmsis_dap_core::backend::Protocol::Swd,
                speed_khz: None,
                target: None,
                under_reset: false,
                core_index: None,
            },
        }
    }
}

/// Build the application router.
pub fn build_router(state: SharedState) -> Router {
    Router::new()
        .merge(api::router())
        .route("/ws", get(ws::ws_handler))
        .fallback(static_or_api_404)
        .with_state(state)
}

/// SPA + static assets; unknown `/api/*` paths return 404 JSON.
async fn static_or_api_404(req: Request) -> Response {
    let path = req.uri().path().to_string();
    if path.starts_with("/api/") {
        (
            StatusCode::NOT_FOUND,
            axum::Json(serde_json::json!({ "error": { "code": "not_found", "message": path } })),
        )
            .into_response()
    } else {
        crate::assets::serve_static(&path).await
    }
}

/// Start the web server and block until shutdown (Ctrl-C).
pub fn serve(backend: Box<dyn Backend>, options: WebServerOptions) -> Result<(), WebError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| WebError::Internal(format!("failed to build runtime: {e}")))?;

    let session = SessionManager::new(backend);
    let executor = spawn(
        session,
        ExecutorConfig {
            allow_destructive: options.allow_destructive,
            flash_timeout: options.flash_timeout,
        },
    );

    let events = executor.events();
    let lease = SessionLease::new(Duration::from_secs(30));
    let upload_dir = tempfile::Builder::new()
        .prefix("cmsis-dap-web-uploads-")
        .tempdir()
        .map_err(|e| WebError::Internal(format!("failed to create upload dir: {e}")))?;
    let state = Arc::new(AppState {
        executor: executor.clone(),
        events,
        lease,
        host: options.host.clone(),
        port: options.port,
        default_connect: options.default_connect,
        uploads: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        _upload_dir: upload_dir,
        symbols: Arc::new(std::sync::RwLock::new(None)),
        debug_info: Arc::new(std::sync::Mutex::new(None)),
        svd: Arc::new(std::sync::RwLock::new(None)),
        watch: Arc::new(std::sync::Mutex::new(Vec::new())),
        monitors: Arc::new(std::sync::Mutex::new(Vec::new())),
        scheduler_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        rtt_task: Arc::new(std::sync::Mutex::new(None)),
        evr_task: Arc::new(std::sync::Mutex::new(None)),
        rtt_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        evr_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    });

    let router = build_router(state.clone());
    let bind = format!("{}:{}", options.host, options.port);
    let listener = runtime
        .block_on(tokio::net::TcpListener::bind(&bind))
        .map_err(|e| WebError::Internal(format!("bind {bind}: {e}")))?;
    let actual_port = listener.local_addr().map_err(|e| WebError::Internal(e.to_string()))?.port();
    println!("CMSIS-DAP Web Debug listening on http://{}:{actual_port}", options.host);

    runtime.block_on(async move {
        // Start live debug schedulers (Live Watch + Peripheral periodic refresh).
        let stop = state.scheduler_stop.clone();
        crate::schedulers::spawn_live_watch(state.clone(), stop);
        crate::schedulers::spawn_peripheral_monitor(state.clone(), state.scheduler_stop.clone());

        let server = axum::serve(listener, router);
        tokio::select! {
            result = server => {
                let _ = result.map_err(|e| WebError::Internal(format!("server error: {e}")));
            }
            _ = tokio::signal::ctrl_c() => {
                // Shut the executor down cleanly.
                let _ = executor.call_async(OperationKind::Disconnect, serde_json::json!({})).await;
            }
        }
        Ok(())
    })
}

