use clap::{Args, Subcommand};

use super::{parse_u32_arg, CliError};
use cmsis_dap_core::error::McpError;
use cmsis_dap_core::session::SessionManager;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct SwoArgs {
    #[command(subcommand)]
    pub action: SwoAction,
}

#[derive(Debug, Subcommand)]
pub enum SwoAction {
    /// Start SWO/SWV trace.
    Start(SwoStartArgs),
    /// Stop SWO/SWV trace.
    Stop,
    /// Poll SWO data in a loop and print to stdout.
    Monitor(SwoMonitorArgs),
}

#[derive(Debug, Args)]
pub struct SwoStartArgs {
    /// SWO baud rate in Hz.
    #[arg(long, value_parser = parse_u32_arg, default_value_t = 2_000_000)]
    pub baud: u32,
    /// TPIU clock frequency in Hz (often the system clock).
    #[arg(long, value_parser = parse_u32_arg, default_value_t = 8_000_000)]
    pub tpiu_clock: u32,
}

#[derive(Debug, Args)]
pub struct SwoMonitorArgs {
    /// Poll interval in milliseconds.
    #[arg(long, value_parser = parse_u32_arg, default_value_t = 100)]
    pub interval_ms: u32,
    /// Number of polls; 0 runs until Ctrl-C.
    #[arg(long, value_parser = parse_u32_arg, default_value_t = 0)]
    pub count: u32,
    /// Write a timestamped log into this directory.
    #[arg(long, value_name = "DIR")]
    pub log_dir: Option<PathBuf>,
    /// Append the timestamped log to this file.
    #[arg(long, value_name = "FILE")]
    pub log_file: Option<PathBuf>,
}

pub fn swo_start(
    session: &mut SessionManager,
    args: &SwoStartArgs,
) -> Result<serde_json::Value, CliError> {
    session.backend().start_swo(args.baud, args.tpiu_clock)?;
    Ok(serde_json::json!({
        "started": true,
        "baud": args.baud,
        "tpiu_clock": args.tpiu_clock,
    }))
}

pub fn swo_stop(session: &mut SessionManager) -> Result<serde_json::Value, CliError> {
    session.backend().stop_swo()?;
    Ok(serde_json::json!({ "stopped": true }))
}

pub fn swo_monitor(
    session: &mut SessionManager,
    args: &SwoMonitorArgs,
    json: bool,
) -> Result<(), CliError> {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut poll_count: u32 = 0;

    loop {
        if args.count > 0 && poll_count >= args.count {
            break;
        }

        match session.backend().read_swo_data() {
            Ok(data) => {
                if !data.is_empty() {
                    if json {
                        let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
                        let line = serde_json::json!({
                            "bytes": data.len(),
                            "data_hex": hex,
                        });
                        writeln!(out, "{line}").ok();
                    } else {
                        let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
                        write!(out, "[{ts}] ").ok();
                        for b in &data {
                            write!(out, "{b:02x} ").ok();
                        }
                        writeln!(out).ok();
                    }
                }
            }
            Err(McpError { code: _, message }) => {
                eprintln!("SWO read error: {message}");
            }
        }

        poll_count = poll_count.saturating_add(1);
        std::thread::sleep(std::time::Duration::from_millis(args.interval_ms as u64));
    }

    Ok(())
}
