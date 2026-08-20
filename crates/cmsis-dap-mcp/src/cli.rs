use clap::Parser;
use std::ffi::OsString;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid protocol: {0}")]
    InvalidProtocol(String),
    #[error(transparent)]
    Clap(#[from] clap::Error),
}

#[derive(Debug, Clone, Parser)]
#[command(name = "cmsis-dap-mcp", about = "CMSIS-DAP MCP server")]
pub struct AppConfig {
    #[arg(long)]
    pub allow_destructive: bool,
    #[arg(long, default_value = "info")]
    pub log_level: String,
    #[arg(long)]
    pub log_file: Option<PathBuf>,
    #[arg(long)]
    pub probe_id: Option<String>,
    #[arg(long)]
    pub protocol: Option<String>,
    #[arg(long)]
    pub speed_khz: Option<u32>,
    #[arg(long)]
    pub target: Option<String>,
    #[arg(long)]
    pub svd: Option<PathBuf>,
    #[arg(long)]
    pub target_yaml: Option<PathBuf>,
    /// Also serve a remote JSON-RPC TCP server on this port (127.0.0.1).
    #[arg(long)]
    pub tcp: Option<u16>,
    /// Also start a GDB server on this port (default 1337 semantics; uses the
    /// same probe via a non-invasive attach).
    #[arg(long)]
    pub gdb_port: Option<u16>,
    /// Path to a JSON config file (keys: allow_destructive, tcp_port,
    /// gdb_port). Loaded at startup and, when present, watched for changes so
    /// edits take effect without a restart. All of its values are overridable
    /// by the CLI flags above and by the runtime update_config tool.
    #[arg(long)]
    pub config_file: Option<PathBuf>,
}

impl AppConfig {
    pub fn parse_from<I, T>(args: I) -> Result<Self, CliError>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let cfg = <AppConfig as Parser>::try_parse_from(args)?;
        if let Some(p) = &cfg.protocol {
            if p != "swd" && p != "jtag" {
                return Err(CliError::InvalidProtocol(p.clone()));
            }
        }
        Ok(cfg)
    }
}
