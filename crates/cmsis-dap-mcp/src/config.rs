//! Server runtime configuration.
//!
//! The MCP server used to require feature-gating flags (`--allow-destructive`,
//! `--tcp`, `--gdb-port`) at process start. That made it impossible to enable
//! or adjust those features once the server was already running. This module
//! defines a [`ServerConfig`] that lives behind a shared, lockable handle so
//! it can be loaded or updated at runtime (via the `update_config` /
//! `reload_config` MCP tools, or a watched config file) without a restart.
//!
//! The config is intentionally small: it only governs the features that were
//! previously gated behind startup flags. Connection/session state stays in
//! [`cmsis_dap_core::session::SessionManager`]; the backend itself is built
//! once at startup and is not part of the mutable runtime config.

use crate::cli::AppConfig;
use serde::Deserialize;
use std::path::Path;

/// Mutable runtime configuration shared by every endpoint (MCP, TCP, GDB).
///
/// The default is the "to-be-configured" state: the server starts fine, all
/// read/write tools are usable, and destructive tools are gated until enabled
/// via `update_config` or the `--allow-destructive` flag.
#[derive(Debug, Clone, Default)]
pub struct ServerConfig {
    /// Gates destructive tools (`erase_flash`, `program_flash`) and
    /// destructive script commands.
    pub allow_destructive: bool,
    /// Remote JSON-RPC TCP server port. `None` disables it.
    pub tcp_port: Option<u16>,
    /// GDB server port. `None` disables it.
    pub gdb_port: Option<u16>,
    /// Source config file (set by `--config-file`). Used by `reload_config`
    /// and the optional auto-reload file watcher. Not read from the file
    /// itself (it would be self-referential).
    pub config_file: Option<std::path::PathBuf>,
}

impl ServerConfig {
    /// Validate the whole config. Returns a human-readable error for the
    /// first problem found so a caller can reject an update atomically
    /// (no partial apply).
    pub fn validate(&self) -> Result<(), String> {
        if let Some(p) = self.tcp_port {
            if p == 0 {
                return Err(format!("tcp_port must be between 1 and 65535, got {p}"));
            }
        }
        if let Some(p) = self.gdb_port {
            if p == 0 {
                return Err(format!("gdb_port must be between 1 and 65535, got {p}"));
            }
        }
        Ok(())
    }

    /// Build the initial config from CLI args, optionally seeded by a config
    /// file. CLI args win over file values when both are present.
    pub fn from_cli(cli: &AppConfig, file: Option<ServerConfigFile>) -> Self {
        let mut cfg = match file {
            Some(f) => ServerConfig {
                allow_destructive: f.allow_destructive,
                tcp_port: f.tcp_port,
                gdb_port: f.gdb_port,
                config_file: None,
            },
            None => ServerConfig::default(),
        };
        if cli.allow_destructive {
            cfg.allow_destructive = true;
        }
        if let Some(p) = cli.tcp {
            cfg.tcp_port = Some(p);
        }
        if let Some(p) = cli.gdb_port {
            cfg.gdb_port = Some(p);
        }
        if let Some(path) = &cli.config_file {
            cfg.config_file = Some(path.clone());
        }
        cfg
    }

    /// Serialize the current config to JSON for tool responses.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "allow_destructive": self.allow_destructive,
            "tcp_port": self.tcp_port,
            "gdb_port": self.gdb_port,
            "config_file": self.config_file.as_ref().map(|p| p.to_string_lossy().to_string()),
        })
    }
}

/// On-disk representation of the server config (JSON). Only the toggleable
/// features are persisted; `config_file` is metadata owned by the process.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ServerConfigFile {
    #[serde(default)]
    pub allow_destructive: bool,
    #[serde(default)]
    pub tcp_port: Option<u16>,
    #[serde(default)]
    pub gdb_port: Option<u16>,
}

/// Read and parse a config file from disk.
///
/// Returns a clear, actionable error string (suitable for an MCP tool error)
/// when the file is missing or malformed, so the caller can surface it
/// instead of failing obscurely.
pub fn load_config_file(path: &Path) -> Result<ServerConfigFile, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read config file {}: {e}", path.display()))?;
    serde_json::from_str::<ServerConfigFile>(&text).map_err(|e| {
        format!(
            "cannot parse config file {}: {e} (expected JSON with optional keys allow_destructive, tcp_port, gdb_port)",
            path.display()
        )
    })
}
