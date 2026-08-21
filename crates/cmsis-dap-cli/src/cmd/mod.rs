pub mod actions;
pub mod chip;
pub mod live;
pub mod option;
pub mod output;
pub mod repl;
pub mod signal;
pub mod swo;
pub mod symbols;

use clap::{Args, Parser, Subcommand};
use cmsis_dap_core::backend::probe_rs::ProbeRsBackend;
use cmsis_dap_core::backend::{AccessWidth, Backend, ConnectOptions, CoreRegister, Protocol};
use cmsis_dap_core::error::{ErrorCode, McpError};
use cmsis_dap_core::security::SecurityPolicy;
use cmsis_dap_core::session::SessionManager;
use serde_json::json;
use std::io::IsTerminal;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("{0}")]
    Mcp(#[from] McpError),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("{0}")]
    Aborted(String),
}

impl CliError {
    /// Exit code convention: 0 ok, 1 runtime error, 2 usage error, 3 aborted
    /// (e.g. stdin read failure in the REPL).
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Aborted(_) => 3,
            CliError::Mcp(_) => 1,
            CliError::InvalidArgument(_) => 2,
        }
    }
}

/// Parse a number that may be decimal or hex (`0x...`).
pub fn parse_u64_arg(s: &str) -> Result<u64, String> {
    let t = s.trim();
    let (radix, digits) = match t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        Some(rest) => (16, rest),
        None => (10, t),
    };
    u64::from_str_radix(digits, radix).map_err(|_| format!("expected a number, got {s}"))
}

fn parse_u32_arg(s: &str) -> Result<u32, String> {
    parse_u64_arg(s).and_then(|v| u32::try_from(v).map_err(|_| format!("number out of range: {s}")))
}

/// Parse an Event Recorder context filter value (0..7).
fn parse_evr_ctx(s: &str) -> Result<u32, String> {
    let value = parse_u32_arg(s)?;
    if value > 7 {
        return Err(format!("evr context must be 0..7, got {s}"));
    }
    Ok(value)
}

pub fn parse_width(s: &str) -> Result<AccessWidth, CliError> {
    match s {
        "u8" => Ok(AccessWidth::U8),
        "u16" => Ok(AccessWidth::U16),
        "u32" => Ok(AccessWidth::U32),
        "u64" => Ok(AccessWidth::U64),
        other => Err(CliError::InvalidArgument(format!(
            "width must be u8/u16/u32/u64, got {other}"
        ))),
    }
}

fn parse_register(s: &str) -> Result<CoreRegister, CliError> {
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
        let number = s
            .parse::<u16>()
            .map_err(|_| CliError::InvalidArgument(format!("register number out of range: {s}")))?;
        Ok(CoreRegister::Number(number))
    } else {
        Ok(CoreRegister::Name(s.to_string()))
    }
}

fn parse_protocol(s: &str) -> Result<Protocol, CliError> {
    match s.to_ascii_lowercase().as_str() {
        "swd" => Ok(Protocol::Swd),
        "jtag" => Ok(Protocol::Jtag),
        other => Err(CliError::InvalidArgument(format!(
            "protocol must be swd or jtag, got {other}"
        ))),
    }
}

fn parse_svd_target(target: &str) -> Result<(String, String, Option<String>), CliError> {
    let parts: Vec<&str> = target.split('.').collect();
    match parts.as_slice() {
        [peripheral, register] => Ok((peripheral.to_string(), register.to_string(), None)),
        [peripheral, register, field] => Ok((
            peripheral.to_string(),
            register.to_string(),
            Some(field.to_string()),
        )),
        _ => Err(CliError::InvalidArgument(format!(
            "svd target must be PERIPH.REG or PERIPH.REG.FIELD, got {target}"
        ))),
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "cmsis-dap-cli",
    version,
    about = "CLI for CMSIS-DAP debug probes (Cortex-M)",
    long_about = "Inspect and control CMSIS-DAP probes and Cortex-M targets: enumerate probes, \
connect over SWD/JTAG, read/write memory and core registers, control execution, use named \
peripherals via SVD files, program flash, run J-Link/OpenOCD style scripts, or enter an \
interactive shell."
)]
pub struct CliArgs {
    /// Probe id or serial to select when multiple probes are connected.
    #[arg(long, global = true)]
    pub probe_id: Option<String>,
    /// Debug wire protocol.
    #[arg(long, global = true, value_parser = ["swd", "jtag"], default_value = "swd")]
    pub protocol: String,
    /// SWD/JTAG clock speed in kHz.
    #[arg(long, global = true)]
    pub speed_khz: Option<u32>,
    /// Target chip name (probe-rs target name).
    #[arg(long, global = true)]
    pub target: Option<String>,
    /// Connect while holding the target reset line (locked/unresponsive targets).
    #[arg(long, global = true)]
    pub under_reset: bool,
    /// Target YAML file with chip/Flash algorithm definitions.
    #[arg(long, global = true, value_name = "FILE")]
    pub target_yaml: Option<PathBuf>,
    /// Keil FLM flash algorithm file - generates target YAML on the fly.
    /// Requires --flash-start, --flash-size, --sram-start, --sram-size.
    /// Mutually exclusive with --target-yaml.
    #[arg(
        long,
        global = true,
        value_name = "FILE",
        conflicts_with = "target_yaml"
    )]
    pub flm: Option<PathBuf>,
    /// Flash start address (required with --flm).
    #[arg(long, global = true, value_parser = parse_u64_arg)]
    pub flash_start: Option<u64>,
    /// Flash size in bytes (required with --flm).
    #[arg(long, global = true, value_parser = parse_u64_arg)]
    pub flash_size: Option<u64>,
    /// SRAM start address (required with --flm).
    #[arg(long, global = true, value_parser = parse_u64_arg)]
    pub sram_start: Option<u64>,
    /// SRAM size in bytes (required with --flm).
    #[arg(long, global = true, value_parser = parse_u64_arg)]
    pub sram_size: Option<u64>,
    /// Core type (default: armv6m, used with --flm).
    #[arg(long, global = true, default_value = "armv6m")]
    pub core: String,
    /// SVD file for named peripheral access (svd subcommands).
    #[arg(long, global = true, value_name = "FILE")]
    pub svd: Option<PathBuf>,
    /// Firmware ELF for symbol resolution (symbols/watch/rtt/evr).
    #[arg(long, global = true, value_name = "FILE")]
    pub elf: Option<PathBuf>,
    /// Print machine-readable JSON instead of human text.
    #[arg(long, global = true)]
    pub json: bool,
    /// Log level (tracing filter), logs go to stderr.
    #[arg(long, global = true, default_value = "warn")]
    pub log_level: String,
    /// Write logs to a file instead of stderr.
    #[arg(long, global = true, value_name = "FILE")]
    pub log_file: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List connected CMSIS-DAP probes.
    List,
    /// Show information about a probe.
    Info,
    /// Connect to a target and show target info.
    Connect,
    /// Disconnect the current session.
    Disconnect,
    /// Show target info (requires a connection).
    Target,
    /// Read memory (optionally export a range to a bin/hex file).
    Read(ReadArgs),
    /// Write memory.
    Write(WriteArgs),
    /// Verify memory against expected values.
    Verify(VerifyArgs),
    /// List available core registers.
    Regs,
    /// Read or write a core register.
    Reg(RegArgs),
    /// Show core status.
    Status,
    /// Halt the core.
    Halt,
    /// Resume the core.
    Resume,
    /// Single-step the core.
    Step,
    /// Reset the target (mode: run or halt).
    Reset(ResetArgs),
    /// Hardware breakpoints.
    Bp(BpArgs),
    /// Watchpoints.
    Wp(WpArgs),
    /// Raw DAP (DP/AP) access.
    Dap(DapArgs),
    /// Named SVD peripheral access.
    Svd(SvdArgs),
    /// Flash erase or program.
    Flash(FlashArgs),
    /// Run a J-Link Commander / OpenOCD style script.
    Script(ScriptArgs),
    /// Chip definition tooling (generate target YAML from a Keil FLM).
    Chip(ChipArgs),
    /// Inspect symbols from a firmware ELF (--elf).
    Symbols(SymbolsArgs),
    /// Poll variables at an address or ELF symbol (live watch).
    Watch(WatchArgs),
    /// J-Link RTT up-channel logging.
    Rtt(RttArgs),
    /// CMSIS-View Event Recorder decoding.
    Evr(EvrArgs),
    /// Non-invasive CPU state snapshot (never resets; restores run state).
    Dump(DumpArgs),
    /// Run a GDB Remote Serial Protocol server.
    GdbServer(GdbServerArgs),
    /// Serve the remote JSON-RPC TCP protocol.
    TcpServer(TcpServerArgs),
    /// SWO/SWV trace control.
    Swo(swo::SwoArgs),
    /// Read/write chip option bytes.
    Option(option::OptionArgs),
    /// Interactive shell (J-Link Commander style commands).
    Repl,
}

#[derive(Debug, Args)]
pub struct ReadArgs {
    #[arg(long, value_parser = parse_u64_arg)]
    pub address: u64,
    #[arg(long, value_parser = ["u8", "u16", "u32", "u64"], default_value = "u32")]
    pub width: String,
    /// Number of elements to read; in export mode, the number of bytes.
    #[arg(long, value_parser = parse_u32_arg, default_value_t = 1)]
    pub count: u32,
    /// Export path (bin/hex). When set, count is the number of bytes.
    #[arg(long, value_name = "FILE")]
    pub output: Option<PathBuf>,
    /// Export format: bin (default) or hex.
    #[arg(long, value_parser = ["bin", "hex"], default_value = "bin")]
    pub format: String,
}

#[derive(Debug, Args)]
pub struct WriteArgs {
    #[arg(long, value_parser = parse_u64_arg)]
    pub address: u64,
    #[arg(long, value_parser = ["u8", "u16", "u32", "u64"], default_value = "u32")]
    pub width: String,
    #[arg(long, value_delimiter = ',', value_parser = parse_u64_arg)]
    pub values: Vec<u64>,
}

#[derive(Debug, Args)]
pub struct VerifyArgs {
    #[arg(long, value_parser = parse_u64_arg)]
    pub address: u64,
    #[arg(long, value_parser = ["u8", "u16", "u32", "u64"], default_value = "u32")]
    pub width: String,
    #[arg(long, value_delimiter = ',', value_parser = parse_u64_arg)]
    pub values: Vec<u64>,
}

#[derive(Debug, Args)]
pub struct RegArgs {
    #[command(subcommand)]
    pub action: RegAction,
}

#[derive(Debug, Subcommand)]
pub enum RegAction {
    Get(RegGetArgs),
    Set(RegSetArgs),
}

#[derive(Debug, Args)]
pub struct RegGetArgs {
    /// Register name (pc, sp, r0, ...) or number.
    pub register: String,
}

#[derive(Debug, Args)]
pub struct RegSetArgs {
    /// Register name (pc, sp, r0, ...) or number.
    pub register: String,
    #[arg(value_parser = parse_u64_arg)]
    pub value: u64,
}

#[derive(Debug, Args)]
pub struct ResetArgs {
    #[arg(long, value_parser = ["run", "halt"], default_value = "run")]
    pub mode: String,
}

#[derive(Debug, Args)]
pub struct BpArgs {
    #[command(subcommand)]
    pub action: BpAction,
}

#[derive(Debug, Subcommand)]
pub enum BpAction {
    Set(BpSetArgs),
    List,
    Clear,
}

#[derive(Debug, Args)]
pub struct BpSetArgs {
    #[arg(value_parser = parse_u64_arg)]
    pub address: u64,
}

#[derive(Debug, Args)]
pub struct WpArgs {
    #[command(subcommand)]
    pub action: WpAction,
}

#[derive(Debug, Subcommand)]
pub enum WpAction {
    Set(WpSetArgs),
    List,
    Clear,
}

#[derive(Debug, Args)]
pub struct WpSetArgs {
    #[arg(value_parser = parse_u64_arg)]
    pub address: u64,
    /// Access type to watch: read, write or rw.
    #[arg(long, value_parser = ["read", "write", "rw"], default_value = "rw")]
    pub access: String,
}

#[derive(Debug, Args)]
pub struct DapArgs {
    #[command(subcommand)]
    pub action: DapAction,
}

#[derive(Debug, Subcommand)]
pub enum DapAction {
    Read(DapReadArgs),
    Write(DapWriteArgs),
}

#[derive(Debug, Args)]
pub struct DapReadArgs {
    #[arg(value_parser = parse_u32_arg)]
    pub address: u32,
}

#[derive(Debug, Args)]
pub struct DapWriteArgs {
    #[arg(value_parser = parse_u32_arg)]
    pub address: u32,
    #[arg(value_parser = parse_u32_arg)]
    pub value: u32,
}

#[derive(Debug, Args)]
pub struct SvdArgs {
    #[command(subcommand)]
    pub action: SvdAction,
}

#[derive(Debug, Subcommand)]
pub enum SvdAction {
    /// List peripherals from the loaded SVD.
    List,
    /// Read PERIPH.REG[.FIELD].
    Read(SvdReadArgs),
    /// Write PERIPH.REG[.FIELD] VALUE.
    Write(SvdWriteArgs),
}

#[derive(Debug, Args)]
pub struct SvdReadArgs {
    pub target: String,
}

#[derive(Debug, Args)]
pub struct SvdWriteArgs {
    pub target: String,
    #[arg(value_parser = parse_u64_arg)]
    pub value: u64,
}

#[derive(Debug, Args)]
pub struct FlashArgs {
    #[command(subcommand)]
    pub action: FlashAction,
}

#[derive(Debug, Subcommand)]
pub enum FlashAction {
    Erase(FlashEraseArgs),
    Program(FlashProgramArgs),
}

#[derive(Debug, Args)]
pub struct FlashEraseArgs {
    #[arg(long, value_parser = parse_u64_arg)]
    pub address: u64,
    #[arg(long, value_parser = parse_u64_arg)]
    pub size: u64,
}

#[derive(Debug, Args)]
pub struct FlashProgramArgs {
    #[arg(long, value_parser = parse_u64_arg)]
    pub address: u64,
    #[arg(long, value_name = "FILE")]
    pub file: PathBuf,
    /// File format: elf, axf, bin or hex (default: inferred from extension).
    #[arg(long, value_parser = ["elf", "axf", "bin", "hex"])]
    pub format: Option<String>,
    /// Read back and verify the programmed data.
    #[arg(long)]
    pub verify: bool,
}

#[derive(Debug, Args)]
pub struct ScriptArgs {
    /// Path of a script file.
    #[arg(long, value_name = "FILE", conflicts_with = "text")]
    pub file: Option<PathBuf>,
    /// Inline script text.
    #[arg(long, conflicts_with = "file")]
    pub text: Option<String>,
}

#[derive(Debug, Args)]
pub struct ChipArgs {
    #[command(subcommand)]
    pub action: ChipAction,
}

#[derive(Debug, Subcommand)]
pub enum ChipAction {
    /// Generate a probe-rs target YAML from a Keil FLM flash algorithm.
    Generate(ChipGenerateArgs),
    /// List all known chip variants (built-in plus --target-yaml chips).
    List,
    /// Search chip variants by name.
    Search(ChipSearchArgs),
}

#[derive(Debug, Args)]
pub struct ChipGenerateArgs {
    /// Keil FLM flash algorithm file (ARM ELF).
    #[arg(long, value_name = "FILE")]
    pub flm: PathBuf,
    /// Flash start address.
    #[arg(long, value_parser = parse_u64_arg)]
    pub flash_start: u64,
    /// Flash size in bytes.
    #[arg(long, value_parser = parse_u64_arg)]
    pub flash_size: u64,
    /// SRAM start address.
    #[arg(long, value_parser = parse_u64_arg)]
    pub sram_start: u64,
    /// SRAM size in bytes.
    #[arg(long, value_parser = parse_u64_arg)]
    pub sram_size: u64,
    /// Chip/variant name used with --target (default: FLM file stem).
    #[arg(long)]
    pub name: Option<String>,
    /// Core type (default: armv6m).
    #[arg(long, default_value = "armv6m")]
    pub core: String,
    /// Output file; use '-' for stdout (default: stdout).
    #[arg(long, value_name = "FILE")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct ChipSearchArgs {
    /// Keyword matched case-insensitively against chip names.
    pub keyword: String,
}

#[derive(Debug, Args)]
pub struct SymbolsArgs {
    #[command(subcommand)]
    pub action: SymbolsAction,
}

#[derive(Debug, Subcommand)]
pub enum SymbolsAction {
    /// List symbols, optionally filtered by a case-insensitive substring.
    List(SymbolsListArgs),
    /// Resolve one symbol name to its address.
    Resolve(SymbolsResolveArgs),
}

#[derive(Debug, Args)]
pub struct SymbolsListArgs {
    /// Optional case-insensitive substring filter.
    pub pattern: Option<String>,
}

#[derive(Debug, Args)]
pub struct SymbolsResolveArgs {
    pub name: String,
}

#[derive(Debug, Args)]
pub struct WatchArgs {
    /// Poll interval in milliseconds.
    #[arg(long, value_parser = parse_u32_arg, default_value_t = 500)]
    pub interval_ms: u32,
    /// Number of samples; 0 runs until Ctrl-C.
    #[arg(long, value_parser = parse_u32_arg, default_value_t = 1)]
    pub count: u32,
    /// Access width applied to every target.
    #[arg(long, value_parser = ["u8", "u16", "u32", "u64"], default_value = "u32")]
    pub width: String,
    /// Targets: symbol names (resolved via --elf) or 0xADDR addresses.
    #[arg(required = true)]
    pub targets: Vec<String>,
    /// Write a timestamped log into this directory (default: current dir).
    #[arg(long, value_name = "DIR", conflicts_with = "log_file")]
    pub log_dir: Option<PathBuf>,
    /// Append the timestamped log to this exact file.
    #[arg(long, value_name = "FILE", conflicts_with = "log_dir")]
    pub log_file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct RttArgs {
    #[command(subcommand)]
    pub action: RttAction,
}

#[derive(Debug, Subcommand)]
pub enum RttAction {
    /// Attach to RTT and list up channels.
    Info,
    /// Poll RTT up channels and print logs.
    Monitor(RttMonitorArgs),
}

#[derive(Debug, Args)]
pub struct RttMonitorArgs {
    /// Comma-separated up channel numbers to monitor.
    #[arg(long, default_value = "0")]
    pub channel: String,
    /// Poll interval in milliseconds.
    #[arg(long, value_parser = parse_u32_arg, default_value_t = 200)]
    pub interval_ms: u32,
    /// Number of polls; 0 runs until Ctrl-C.
    #[arg(long, value_parser = parse_u32_arg, default_value_t = 0)]
    pub count: u32,
    /// RTT control block address override (default: --elf _SEGGER_RTT, then RAM scan).
    #[arg(long, value_parser = parse_u64_arg)]
    pub address: Option<u64>,
    /// Maximum bytes to read per channel per poll.
    #[arg(long, value_parser = parse_u32_arg, default_value_t = 1024)]
    pub max_bytes: u32,
    /// Write a timestamped log into this directory (default: current dir).
    #[arg(long, value_name = "DIR", conflicts_with = "log_file")]
    pub log_dir: Option<PathBuf>,
    /// Append the timestamped log to this exact file.
    #[arg(long, value_name = "FILE", conflicts_with = "log_dir")]
    pub log_file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct EvrArgs {
    #[command(subcommand)]
    pub action: EvrAction,
}

#[derive(Debug, Subcommand)]
pub enum EvrAction {
    /// Attach to the Event Recorder and show its state.
    Info,
    /// Poll committed events and print decoded records.
    Monitor(EvrMonitorArgs),
}

#[derive(Debug, Args)]
pub struct EvrMonitorArgs {
    /// Poll interval in milliseconds.
    #[arg(long, value_parser = parse_u32_arg, default_value_t = 200)]
    pub interval_ms: u32,
    /// Number of polls; 0 runs until Ctrl-C.
    #[arg(long, value_parser = parse_u32_arg, default_value_t = 0)]
    pub count: u32,
    /// Event context filter, 0..7 (repeatable or comma list); default: all.
    #[arg(long, value_delimiter = ',', value_parser = parse_evr_ctx)]
    pub ctx: Vec<u32>,
    /// EventRecorderInfo address override (default: --elf EventRecorderInfo).
    #[arg(long, value_parser = parse_u64_arg)]
    pub address: Option<u64>,
    /// Write a timestamped log into this directory (default: current dir).
    #[arg(long, value_name = "DIR", conflicts_with = "log_file")]
    pub log_dir: Option<PathBuf>,
    /// Append the timestamped log to this exact file.
    #[arg(long, value_name = "FILE", conflicts_with = "log_dir")]
    pub log_file: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct DumpArgs {
    /// Addresses to sample (0xADDR or symbol names resolved via --elf); repeatable.
    #[arg(long = "address", value_name = "ADDR")]
    pub addresses: Vec<String>,
    /// Number of words to dump from the top of the MSP/PSP stacks.
    #[arg(long, value_parser = parse_u32_arg, default_value_t = 16)]
    pub stack_words: u32,
    /// Do not restore the previous run state after the dump.
    #[arg(long)]
    pub no_restore: bool,
}

#[derive(Debug, Args)]
pub struct GdbServerArgs {
    /// TCP port for the GDB stub (default 1337).
    #[arg(long, value_parser = parse_u32_arg, default_value_t = 1337)]
    pub port: u32,
    /// Reset and halt the core after attach instead of just attaching.
    #[arg(long)]
    pub reset_halt: bool,
}

#[derive(Debug, Args)]
pub struct TcpServerArgs {
    /// TCP port for the remote JSON-RPC server (default 4000).
    #[arg(long, value_parser = parse_u32_arg, default_value_t = 4000)]
    pub port: u32,
}

#[derive(Debug, Clone)]
pub struct ReplOptions {
    pub json: bool,
    pub probe_id: Option<String>,
    pub protocol: String,
    pub speed_khz: Option<u32>,
    pub target: Option<String>,
    pub under_reset: bool,
    pub elf: Option<PathBuf>,
}

impl Default for ReplOptions {
    fn default() -> Self {
        Self {
            json: false,
            probe_id: None,
            protocol: "swd".into(),
            speed_khz: None,
            target: None,
            under_reset: false,
            elf: None,
        }
    }
}

/// Build the probe-rs backend, optionally loading a target YAML registry.
pub fn make_backend(target_yaml: Option<&std::path::Path>) -> Result<Box<dyn Backend>, McpError> {
    match target_yaml {
        Some(path) => Ok(Box::new(ProbeRsBackend::with_registry(
            cmsis_dap_core::backend::probe_rs::registry_from_yaml(path)?,
        ))),
        None => Ok(Box::new(ProbeRsBackend::new())),
    }
}

/// Global connection/output options, captured before the subcommand is moved
/// out of `CliArgs` (avoids borrow-after-partial-move).
pub(crate) struct Globals {
    probe_id: Option<String>,
    protocol: String,
    speed_khz: Option<u32>,
    target: Option<String>,
    under_reset: bool,
    target_yaml: Option<PathBuf>,
    svd: Option<PathBuf>,
    elf: Option<PathBuf>,
    json: bool,
}

fn connect(
    globals: &Globals,
    session: &mut SessionManager,
) -> Result<cmsis_dap_core::backend::TargetInfo, CliError> {
    let target = resolve_target(globals)?;
    let opts = ConnectOptions {
        probe_id: globals.probe_id.clone(),
        protocol: parse_protocol(&globals.protocol)?,
        speed_khz: globals.speed_khz,
        target,
        under_reset: globals.under_reset,
    };
    Ok(session.connect(&opts)?)
}

/// Resolve the target chip name: use `--target` when given, otherwise
/// auto-select the single variant from `--target-yaml` (or fail with a clear
/// error when the file defines several).
fn resolve_target(globals: &Globals) -> Result<Option<String>, CliError> {
    if let Some(target) = &globals.target {
        return Ok(Some(target.clone()));
    }
    let Some(path) = &globals.target_yaml else {
        return Ok(None);
    };
    let variants = cmsis_dap_core::backend::probe_rs::target_yaml_variants(path)?;
    match variants.as_slice() {
        [single] => Ok(Some(single.clone())),
        [] => Err(CliError::InvalidArgument(format!(
            "no chip variants found in {}",
            path.display()
        ))),
        many => Err(CliError::InvalidArgument(format!(
            "--target is required; target yaml defines multiple variants: {}",
            many.join(", ")
        ))),
    }
}

fn load_svd(globals: &Globals, session: &mut SessionManager) -> Result<(), CliError> {
    match &globals.svd {
        Some(path) => {
            session.load_svd(path)?;
            Ok(())
        }
        None => Err(CliError::Mcp(McpError::new(
            ErrorCode::SvdNotLoaded,
            "provide an SVD file with --svd",
        ))),
    }
}

/// Execute a parsed CLI invocation against the given backend and return the
/// structured result (mirroring MCP tool payloads). `Ok(None)` means the
/// command produced no top-level output (interactive REPL).
pub fn run(
    args: CliArgs,
    backend: Box<dyn Backend>,
) -> Result<Option<serde_json::Value>, CliError> {
    let mut session = SessionManager::new(backend);
    let globals = Globals {
        probe_id: args.probe_id.clone(),
        protocol: args.protocol.clone(),
        speed_khz: args.speed_khz,
        target: args.target.clone(),
        under_reset: args.under_reset,
        target_yaml: args.target_yaml.clone(),
        svd: args.svd.clone(),
        elf: args.elf.clone(),
        json: args.json,
    };
    match args.command {
        Command::List => {
            let probes = session.backend().list_probes()?;
            Ok(Some(json!({ "probes": probes })))
        }
        Command::Info => {
            let probes = session.backend().list_probes()?;
            let probe = match &globals.probe_id {
                Some(id) => probes
                    .iter()
                    .find(|p| p.id == *id || p.serial.as_deref() == Some(id.as_str()))
                    .cloned(),
                None => probes.first().cloned(),
            };
            match probe {
                Some(p) => Ok(Some(json!({ "probe": p }))),
                None => Err(CliError::Mcp(McpError::new(
                    ErrorCode::ProbeNotFound,
                    "no matching probe found",
                ))),
            }
        }
        Command::Connect => {
            let info = connect(&globals, &mut session)?;
            Ok(Some(json!({ "target": info })))
        }
        Command::Disconnect => {
            session.disconnect()?;
            Ok(Some(json!({ "disconnected": true })))
        }
        Command::Target => {
            connect(&globals, &mut session)?;
            session.ensure_connected()?;
            let info = session
                .target_info()
                .expect("connected target has info")
                .clone();
            Ok(Some(json!({ "target": info })))
        }
        Command::Read(a) => {
            connect(&globals, &mut session)?;
            Ok(Some(actions::read(&mut session, &a)?))
        }
        Command::Write(a) => {
            connect(&globals, &mut session)?;
            Ok(Some(actions::write(&mut session, &a)?))
        }
        Command::Verify(a) => {
            connect(&globals, &mut session)?;
            Ok(Some(actions::verify(&mut session, &a)?))
        }
        Command::Regs => {
            connect(&globals, &mut session)?;
            Ok(Some(actions::regs(&mut session)?))
        }
        Command::Reg(a) => {
            connect(&globals, &mut session)?;
            Ok(Some(actions::reg(&mut session, &a)?))
        }
        Command::Status => {
            connect(&globals, &mut session)?;
            Ok(Some(actions::status(&mut session)?))
        }
        Command::Halt => {
            connect(&globals, &mut session)?;
            session.backend().halt()?;
            Ok(Some(json!({ "halted": true })))
        }
        Command::Resume => {
            connect(&globals, &mut session)?;
            session.backend().resume()?;
            Ok(Some(json!({ "running": true })))
        }
        Command::Step => {
            connect(&globals, &mut session)?;
            session.backend().step()?;
            Ok(Some(json!({ "stepped": true })))
        }
        Command::Reset(a) => {
            connect(&globals, &mut session)?;
            Ok(Some(actions::reset(&mut session, &a)?))
        }
        Command::Bp(a) => {
            connect(&globals, &mut session)?;
            Ok(Some(actions::bp(&mut session, &a)?))
        }
        Command::Wp(a) => {
            connect(&globals, &mut session)?;
            Ok(Some(actions::wp(&mut session, &a)?))
        }
        Command::Dap(a) => {
            connect(&globals, &mut session)?;
            Ok(Some(actions::dap(&mut session, &a)?))
        }
        Command::Svd(a) => {
            connect(&globals, &mut session)?;
            load_svd(&globals, &mut session)?;
            Ok(Some(actions::svd(&mut session, &a)?))
        }
        Command::Flash(a) => {
            connect(&globals, &mut session)?;
            Ok(Some(actions::flash(&mut session, &a)?))
        }
        Command::Script(a) => {
            let text = match (&a.file, &a.text) {
                (Some(path), None) => std::fs::read_to_string(path).map_err(|e| {
                    CliError::Mcp(McpError::new(
                        ErrorCode::FileError,
                        format!("failed to read script {}: {e}", path.display()),
                    ))
                })?,
                (None, Some(text)) => text.clone(),
                _ => {
                    return Err(CliError::InvalidArgument(
                        "script requires exactly one of --file or --text".into(),
                    ))
                }
            };
            let policy = SecurityPolicy {
                allow_destructive: true,
            };
            let mut engine = cmsis_dap_core::script::ScriptEngine::with_connection(
                policy,
                globals.probe_id.clone(),
                parse_protocol(&globals.protocol)?,
                globals.speed_khz,
                globals.target.clone(),
                globals.under_reset,
            );
            let report = engine.run_script(&mut session, &text)?;
            if !report.ok {
                let detail = report
                    .results
                    .iter()
                    .find(|r| r.status == "error")
                    .and_then(|r| r.output.get("message").and_then(|m| m.as_str()))
                    .unwrap_or("script failed");
                return Err(CliError::Mcp(McpError::new(
                    ErrorCode::InternalError,
                    format!("script failed: {detail}"),
                )));
            }
            Ok(Some(json!(report)))
        }
        Command::Chip(a) => match a.action {
            ChipAction::Generate(g) => Ok(Some(actions::chip_generate(&g)?)),
            ChipAction::List => Ok(Some(actions::chip_list(&globals)?)),
            ChipAction::Search(s) => Ok(Some(actions::chip_search(&globals, &s)?)),
        },
        Command::Symbols(a) => match a.action {
            SymbolsAction::List(l) => Ok(Some(live::symbols_list(
                globals.elf.as_deref(),
                l.pattern.as_deref(),
            )?)),
            SymbolsAction::Resolve(r) => Ok(Some(live::symbols_resolve(
                globals.elf.as_deref(),
                &r.name,
            )?)),
        },
        Command::Watch(a) => {
            connect(&globals, &mut session)?;
            let symbols = match globals.elf.as_deref() {
                Some(path) => Some(symbols::load_symbols(path)?),
                None => None,
            };
            let width = parse_width(&a.width)?;
            let mut items = Vec::new();
            for target in &a.targets {
                let (address, label) = live::resolve_target(target, symbols.as_ref())?;
                items.push(live::WatchItem {
                    label,
                    address,
                    width,
                });
            }
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            live::watch_run(
                &mut session,
                &items,
                a.interval_ms,
                a.count,
                globals.json,
                &mut out,
                live::LogTarget::from_args(a.log_dir, a.log_file),
            )?;
            Ok(None)
        }
        Command::Rtt(a) => match a.action {
            RttAction::Info => {
                connect(&globals, &mut session)?;
                Ok(Some(live::rtt_info(
                    &mut session,
                    globals.elf.as_deref(),
                    None,
                )?))
            }
            RttAction::Monitor(m) => {
                connect(&globals, &mut session)?;
                let stdout = std::io::stdout();
                let mut out = stdout.lock();
                live::rtt_monitor(
                    &mut session,
                    &m,
                    globals.elf.as_deref(),
                    globals.json,
                    &mut out,
                )?;
                Ok(None)
            }
        },
        Command::Evr(a) => match a.action {
            EvrAction::Info => {
                connect(&globals, &mut session)?;
                Ok(Some(live::evr_info(
                    &mut session,
                    globals.elf.as_deref(),
                    None,
                )?))
            }
            EvrAction::Monitor(m) => {
                connect(&globals, &mut session)?;
                let stdout = std::io::stdout();
                let mut out = stdout.lock();
                live::evr_monitor(
                    &mut session,
                    &m,
                    globals.elf.as_deref(),
                    globals.json,
                    &mut out,
                )?;
                Ok(None)
            }
        },
        Command::Dump(a) => {
            connect(&globals, &mut session)?;
            Ok(Some(actions::dump(
                &mut session,
                globals.elf.as_deref(),
                &a,
            )?))
        }
        Command::GdbServer(a) => {
            let options = cmsis_dap_core::gdb::GdbServerOptions {
                probe_id: globals.probe_id.clone(),
                protocol: Some(parse_protocol(&globals.protocol)?),
                speed_khz: globals.speed_khz,
                target: globals.target.clone(),
                target_yaml: globals.target_yaml.clone(),
                reset_halt: a.reset_halt,
            };
            cmsis_dap_core::gdb::connect_and_serve(
                options,
                Some(&format!("127.0.0.1:{}", a.port)),
            )?;
            Ok(None)
        }
        Command::TcpServer(a) => {
            let shared = std::sync::Arc::new(std::sync::Mutex::new(session));
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    CliError::Mcp(McpError::new(
                        ErrorCode::InternalError,
                        format!("failed to start tokio runtime: {e}"),
                    ))
                })?;
            runtime.block_on(cmsis_dap_core::remote::serve(
                &shared,
                &format!("127.0.0.1:{}", a.port),
            ))?;
            Ok(None)
        }
        Command::Swo(a) => {
            connect(&globals, &mut session)?;
            match a.action {
                swo::SwoAction::Start(s) => Ok(Some(swo::swo_start(&mut session, &s)?)),
                swo::SwoAction::Stop => Ok(Some(swo::swo_stop(&mut session)?)),
                swo::SwoAction::Monitor(m) => {
                    swo::swo_monitor(&mut session, &m, globals.json)?;
                    Ok(None)
                }
            }
        }
        Command::Option(a) => {
            connect(&globals, &mut session)?;
            match a.action {
                option::OptionAction::Read => Ok(Some(option::option_read(&mut session)?)),
                option::OptionAction::Write(w) => Ok(Some(option::option_write(&mut session, &w)?)),
            }
        }
        Command::Repl => {
            let stdin = std::io::stdin();
            let interactive = stdin.is_terminal();
            let mut reader = stdin.lock();
            let opts = ReplOptions {
                json: globals.json,
                probe_id: globals.probe_id.clone(),
                protocol: globals.protocol.clone(),
                speed_khz: globals.speed_khz,
                target: globals.target.clone(),
                under_reset: globals.under_reset,
                elf: globals.elf.clone(),
            };
            repl::run(&opts, &mut session, &mut reader, interactive)?;
            Ok(None)
        }
    }
}
