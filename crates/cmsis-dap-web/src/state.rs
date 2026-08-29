//! Shared application state handed to every axum handler.

use crate::executor::ExecutorHandle;
use crate::op::ServerEvent;
use crate::session::SessionLease;
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct AppState {
    pub executor: ExecutorHandle,
    pub events: broadcast::Sender<ServerEvent>,
    pub lease: Arc<SessionLease>,
    pub host: String,
    pub port: u16,
    /// Default connect options supplied by the CLI (`--probe-id` etc.).
    pub default_connect: cmsis_dap_core::backend::ConnectOptions,
    /// Uploaded firmware files: file_id -> absolute path. The owning TempDir
    /// keeps the directory alive for the server lifetime and removes it on
    /// shutdown.
    pub uploads: Arc<std::sync::Mutex<std::collections::HashMap<String, std::path::PathBuf>>>,
    pub _upload_dir: tempfile::TempDir,
}

pub type SharedState = Arc<AppState>;

