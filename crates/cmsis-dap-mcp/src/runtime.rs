//! Server runtime: owns the shared [`ServerConfig`] and the running
//! TCP/GDB server tasks, and keeps them in sync.
//!
//! Every config change funnels through [`ServerRuntime::reconcile`], which
//! compares the *desired* state (the config) against the *actual* state (the
//! running tasks) and starts/stops servers as needed. This single
//! reconciliation point is what keeps the system consistent across the
//! startup path, the `update_config` tool, the `reload_config` tool, and the
//! optional file watcher.

use crate::config::{ServerConfig, ServerConfigFile};
use cmsis_dap_core::gdb::{connect_and_serve, GdbServerOptions};
use cmsis_dap_core::remote;
use cmsis_dap_core::session::SessionManager;
use std::sync::{Arc, Mutex, RwLock};

/// A running remote TCP server task.
pub struct TcpServerState {
    pub handle: tokio::task::JoinHandle<()>,
    pub port: u16,
}

/// A running GDB server thread. GDB's serve loop blocks on the runtime thread,
/// so it is spawned on a dedicated OS thread and (unlike the TCP task) cannot
/// be cleanly aborted from outside.
pub struct GdbServerState {
    pub handle: std::thread::JoinHandle<()>,
    pub port: u16,
}

pub struct ServerRuntime {
    pub config: Arc<RwLock<ServerConfig>>,
    pub session: Arc<Mutex<SessionManager>>,
    tcp: Mutex<Option<TcpServerState>>,
    gdb: Mutex<Option<GdbServerState>>,
    gdb_options: GdbServerOptions,
}

impl ServerRuntime {
    /// Build a runtime for tests / simple use: a session and the
    /// `allow_destructive` flag, with no servers and default GDB options.
    pub fn new(session: SessionManager, allow_destructive: bool) -> Arc<Self> {
        Arc::new(Self {
            config: Arc::new(RwLock::new(ServerConfig {
                allow_destructive,
                ..Default::default()
            })),
            session: Arc::new(Mutex::new(session)),
            tcp: Mutex::new(None),
            gdb: Mutex::new(None),
            gdb_options: GdbServerOptions::default(),
        })
    }

    /// Build a runtime sharing an existing session and a fully specified
    /// config (used by `main`, which seeds config from CLI/file and supplies
    /// GDB connection options).
    pub fn from_session(
        session: Arc<Mutex<SessionManager>>,
        config: ServerConfig,
        gdb_options: GdbServerOptions,
    ) -> Arc<Self> {
        Arc::new(Self {
            config: Arc::new(RwLock::new(config)),
            session,
            tcp: Mutex::new(None),
            gdb: Mutex::new(None),
            gdb_options,
        })
    }

    /// Reconcile running servers with the desired config. Idempotent: calling
    /// it when nothing changed is a no-op. Safe to call from any config path.
    pub fn reconcile(&self) {
        self.reconcile_tcp();
        self.reconcile_gdb();
    }

    fn reconcile_tcp(&self) {
        let desired = self.config.read().unwrap().tcp_port;
        let mut guard = self.tcp.lock().unwrap();
        let current = guard.as_ref().map(|s| s.port);
        if desired == current {
            return;
        }
        // Stop the existing task if its port differs (or is being disabled).
        if let Some(state) = guard.take() {
            state.handle.abort();
        }
        if let Some(port) = desired {
            let session = Arc::clone(&self.session);
            let bind = format!("127.0.0.1:{port}");
            let handle = tokio::spawn(async move {
                if let Err(e) = remote::serve(&session, &bind).await {
                    tracing::warn!("remote TCP server error: {e}");
                }
            });
            *guard = Some(TcpServerState { handle, port });
            tracing::info!("started remote TCP server on 127.0.0.1:{port}");
        }
    }

    fn reconcile_gdb(&self) {
        let desired = self.config.read().unwrap().gdb_port;
        let mut guard = self.gdb.lock().unwrap();
        let current = guard.as_ref().map(|s| s.port);
        if desired == current {
            return;
        }
        // A GDB server thread cannot be cleanly aborted. If one is already
        // running we keep it and ask for a restart to change the port.
        if guard.is_some() {
            tracing::warn!(
                "GDB server already running on port {}; to change its port, restart the server",
                current.unwrap()
            );
            return;
        }
        if let Some(port) = desired {
            let options = self.gdb_options.clone();
            let bind = format!("127.0.0.1:{port}");
            let handle = std::thread::spawn(move || {
                if let Err(e) = connect_and_serve(options, Some(&bind)) {
                    tracing::warn!("GDB server error: {e}");
                }
            });
            *guard = Some(GdbServerState { handle, port });
            tracing::info!("started GDB server on 127.0.0.1:{port}");
        }
    }

    /// Apply a partial update from the `update_config` tool.
    ///
    /// Fields that are `None` are left unchanged. The candidate config is
    /// validated *before* anything is written, so an invalid value rejects
    /// the whole update and leaves the running config untouched (atomic).
    /// Returns the new config on success.
    pub fn update(
        &self,
        allow_destructive: Option<bool>,
        tcp_port: Option<u16>,
        gdb_port: Option<u16>,
    ) -> Result<ServerConfig, String> {
        let candidate = {
            let cur = self.config.read().unwrap();
            let mut c = cur.clone();
            if let Some(v) = allow_destructive {
                c.allow_destructive = v;
            }
            if let Some(v) = tcp_port {
                c.tcp_port = Some(v);
            }
            if let Some(v) = gdb_port {
                c.gdb_port = Some(v);
            }
            c
        };
        candidate.validate()?;
        {
            let mut w = self.config.write().unwrap();
            *w = candidate.clone();
        }
        self.reconcile();
        Ok(candidate)
    }

    /// Re-read the config file supplied at startup (`--config-file`) and apply
    /// it. Fails clearly (without changing anything) when no file was given,
    /// the file is missing, or the contents are invalid.
    pub fn apply_config_file(&self) -> Result<ServerConfig, String> {
        let path = {
            let cfg = self.config.read().unwrap();
            cfg.config_file.clone()
        };
        let path = match path {
            Some(p) => p,
            None => {
                return Err(
                    "no config file was provided at startup; pass --config-file, or use update_config to configure at runtime".into(),
                )
            }
        };
        let file: ServerConfigFile = load_from_file(&path)?;
        let candidate = {
            let cur = self.config.read().unwrap();
            ServerConfig {
                allow_destructive: file.allow_destructive,
                tcp_port: file.tcp_port,
                gdb_port: file.gdb_port,
                // Keep the file path itself; do not overwrite it from the file.
                config_file: cur.config_file.clone(),
            }
        };
        candidate.validate()?;
        {
            let mut w = self.config.write().unwrap();
            *w = candidate.clone();
        }
        self.reconcile();
        Ok(candidate)
    }
}

fn load_from_file(path: &std::path::Path) -> Result<ServerConfigFile, String> {
    crate::config::load_config_file(path)
}

/// Spawn a background task that watches `--config-file` and re-applies it on
/// change, so edits take effect without restarting the server.
///
/// Failures are non-fatal: if watching cannot be set up, the server keeps
/// running and the `reload_config` tool remains available as a manual fallback.
#[cfg(feature = "config-watch")]
pub fn spawn_config_watcher(runtime: Arc<ServerRuntime>, path: std::path::PathBuf) {
    use notify::{Config as NotifyConfig, Event, RecommendedWatcher, RecursiveMode, Watcher};

    tokio::spawn(async move {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<notify::Result<Event>>(32);
        let mut watcher = match RecommendedWatcher::new(
            move |res| {
                let _ = tx.blocking_send(res);
            },
            NotifyConfig::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!("config file watcher unavailable: {e}");
                return;
            }
        };
        if let Err(e) = watcher.watch(&path, RecursiveMode::NonRecursive) {
            tracing::warn!("cannot watch config file {}: {e}", path.display());
            return;
        }
        tracing::info!("watching config file {} for changes", path.display());
        while let Some(res) = rx.recv().await {
            if res.is_ok() {
                match runtime.apply_config_file() {
                    Ok(cfg) => tracing::info!("auto-reloaded config: {cfg:?}"),
                    Err(e) => tracing::warn!("auto-reload config failed: {e}"),
                }
            }
        }
    });
}
