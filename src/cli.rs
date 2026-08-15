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
