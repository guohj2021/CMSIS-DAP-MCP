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
    /// Loaded ELF/AXF symbol database (V0.1: one active image).
    pub symbols: Arc<std::sync::RwLock<Option<cmsis_dap_core::symbols::SymbolDatabase>>>,
    /// Loaded DWARF debug info (source locations); None when unavailable.
    pub debug_info: Arc<std::sync::Mutex<Option<cmsis_dap_core::symbols::DebugInfo>>>,
    /// Loaded SVD database (one active).
    pub svd: Arc<std::sync::RwLock<Option<cmsis_dap_core::svd::SvdDatabase>>>,
    /// Watch items.
    pub watch: Arc<std::sync::Mutex<Vec<crate::watch::WatchItem>>>,
    /// Peripheral monitor items.
    pub monitors: Arc<std::sync::Mutex<Vec<crate::monitor::MonitorItem>>>,
    /// Stop flags for the live-watch / peripheral schedulers.
    pub scheduler_stop: Arc<std::sync::atomic::AtomicBool>,
    /// Active RTT / EVR monitor task handles (None when stopped).
    pub rtt_task: Arc<std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub evr_task: Arc<std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub rtt_stop: Arc<std::sync::atomic::AtomicBool>,
    pub evr_stop: Arc<std::sync::atomic::AtomicBool>,
}

pub type SharedState = Arc<AppState>;

