//! Live debugging features: symbol lookup, variable watch, RTT and the
//! CMSIS-View Event Recorder, all with timestamped log export.

use super::symbols;
use super::{parse_u64_arg, CliError, EvrMonitorArgs, RttMonitorArgs};
use crate::cmd::signal;
use cmsis_dap_core::backend::{AccessWidth, EvrEvent};
use cmsis_dap_core::error::{ErrorCode, McpError};
use cmsis_dap_core::session::SessionManager;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Host capture timestamp as (display text, RFC 3339 with millis).
pub fn host_now() -> (String, String) {
    let now = chrono::Local::now();
    (
        now.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
        now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    )
}

fn file_error(msg: impl Into<String>) -> CliError {
    CliError::Mcp(McpError::new(ErrorCode::FileError, msg))
}

/// Where monitor output is written.
pub enum LogTarget {
    /// Directory; a unique `{prefix}-{unix}.log` file is created inside it.
    Dir(PathBuf),
    /// Exact file path, appended.
    File(PathBuf),
}

impl LogTarget {
    /// `--log-dir` wins over the default current directory; `--log-file`
    /// overrides both (clap enforces mutual exclusion).
    pub fn from_args(log_dir: Option<PathBuf>, log_file: Option<PathBuf>) -> Self {
        if let Some(file) = log_file {
            Self::File(file)
        } else {
            Self::Dir(log_dir.unwrap_or_else(|| PathBuf::from(".")))
        }
    }

    pub fn open(&self, prefix: &str) -> Result<Option<(File, PathBuf)>, CliError> {
        match self {
            Self::File(path) => {
                let file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .map_err(|e| {
                        file_error(format!("failed to open log file {}: {e}", path.display()))
                    })?;
                Ok(Some((file, path.clone())))
            }
            Self::Dir(dir) => {
                if !dir.exists() {
                    std::fs::create_dir_all(dir).map_err(|e| {
                        file_error(format!(
                            "failed to create log directory {}: {e}",
                            dir.display()
                        ))
                    })?;
                }
                let seconds = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let path = dir.join(format!("{prefix}-{seconds}.log"));
                let file = File::create(&path).map_err(|e| {
                    file_error(format!("failed to create log file {}: {e}", path.display()))
                })?;
                Ok(Some((file, path)))
            }
        }
    }
}

/// Writes monitor lines to stdout and (when enabled) the log file.
pub struct Monitor<'a> {
    json: bool,
    stdout: &'a mut dyn Write,
    log: Option<BufWriter<File>>,
}

impl<'a> Monitor<'a> {
    pub fn new(json: bool, stdout: &'a mut dyn Write, log: Option<(File, PathBuf)>) -> Self {
        Self {
            json,
            stdout,
            log: log.map(|(f, _)| BufWriter::new(f)),
        }
    }

    /// Emit one line: `json_value` in `--json` mode, `text` otherwise. The
    /// exported log file receives exactly the same line.
    pub fn emit(&mut self, text: String, json_value: Option<Value>) -> Result<(), CliError> {
        let line = match (self.json, json_value) {
            (true, Some(value)) => serde_json::to_string(&value)
                .map_err(|e| file_error(format!("failed to serialize monitor output: {e}")))?,
            _ => text,
        };
        writeln!(self.stdout, "{line}")
            .map_err(|e| file_error(format!("failed to write monitor output: {e}")))?;
        if let Some(log) = &mut self.log {
            writeln!(log, "{line}")
                .map_err(|e| file_error(format!("failed to write monitor log: {e}")))?;
            log.flush()
                .map_err(|e| file_error(format!("failed to flush monitor log: {e}")))?;
        }
        Ok(())
    }
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02X}")).collect()
}

fn width_digits(width: AccessWidth) -> usize {
    match width {
        AccessWidth::U8 => 2,
        AccessWidth::U16 => 4,
        AccessWidth::U32 => 8,
        AccessWidth::U64 => 16,
    }
}

fn width_name(width: AccessWidth) -> &'static str {
    match width {
        AccessWidth::U8 => "u8",
        AccessWidth::U16 => "u16",
        AccessWidth::U32 => "u32",
        AccessWidth::U64 => "u64",
    }
}

/// One watched variable.
#[derive(Debug, Clone)]
pub struct WatchItem {
    pub label: String,
    pub address: u64,
    pub width: AccessWidth,
}

/// REPL watch state (items plus refresh interval).
#[derive(Debug, Clone, Default)]
pub struct WatchState {
    pub items: Vec<WatchItem>,
    pub interval_ms: u32,
}

impl WatchState {
    pub fn add(
        &mut self,
        target: &str,
        width: AccessWidth,
        label: Option<String>,
        symbols: Option<&BTreeMap<String, u64>>,
    ) -> Result<(), CliError> {
        let (address, resolved_label) = resolve_target(target, symbols)?;
        self.items.push(WatchItem {
            label: label.unwrap_or(resolved_label),
            address,
            width,
        });
        Ok(())
    }

    pub fn remove(&mut self, selector: &str) -> Result<(), CliError> {
        let index = if selector.chars().all(|c| c.is_ascii_digit()) {
            selector.parse::<usize>().map_err(|_| {
                CliError::InvalidArgument(format!("invalid watch index: {selector}"))
            })?
        } else {
            self.items
                .iter()
                .position(|i| i.label == selector)
                .ok_or_else(|| {
                    CliError::InvalidArgument(format!("no watched variable named '{selector}'"))
                })?
        };
        if index >= self.items.len() {
            return Err(CliError::InvalidArgument(format!(
                "watch index {index} out of range ({} items)",
                self.items.len()
            )));
        }
        self.items.remove(index);
        Ok(())
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

pub fn resolve_target(
    target: &str,
    symbols: Option<&BTreeMap<String, u64>>,
) -> Result<(u64, String), CliError> {
    if let Ok(address) = parse_u64_arg(target) {
        return Ok((address, target.to_string()));
    }
    let symbols = symbols.ok_or_else(|| {
        CliError::InvalidArgument(format!(
            "'{target}' looks like a symbol name; provide a firmware ELF with --elf"
        ))
    })?;
    let address = symbols.get(target).copied().ok_or_else(|| {
        CliError::Mcp(McpError::new(
            ErrorCode::FileError,
            format!("symbol '{target}' not found in the firmware ELF"),
        ))
    })?;
    Ok((address, target.to_string()))
}

/// Poll the given watch items until `count` samples (0 = until Ctrl-C).
/// Returns the number of completed polls.
pub fn watch_run(
    session: &mut SessionManager,
    items: &[WatchItem],
    interval_ms: u32,
    count: u32,
    json: bool,
    stdout: &mut dyn Write,
    log: LogTarget,
) -> Result<u32, CliError> {
    if items.is_empty() {
        return Err(CliError::InvalidArgument(
            "no variables to watch; use 'watch add <target>' first".into(),
        ));
    }
    let opened = log.open("watch")?;
    if let Some((_, path)) = &opened {
        eprintln!("logging to {}", path.display());
    }
    let mut monitor = Monitor::new(json, stdout, opened);
    signal::install();
    signal::reset();
    let mut polls = 0u32;
    loop {
        if signal::interrupted() {
            eprintln!("stopped (Ctrl-C)");
            break;
        }
        let (host_text, host_rfc) = host_now();
        for item in items {
            let values = session
                .backend()
                .read_memory(item.address, item.width, 1)
                .map_err(|e| {
                    CliError::Mcp(McpError::new(
                        e.code,
                        format!(
                            "failed to read {} at 0x{:x}: {}",
                            item.label, item.address, e.message
                        ),
                    ))
                })?;
            let value = values.first().copied().unwrap_or(0);
            let text = format!(
                "[{host_text}] {} = 0x{:0width$X}",
                item.label,
                value,
                width = width_digits(item.width)
            );
            let payload = json!({
                "host_ts": host_rfc,
                "target": item.label,
                "address": item.address,
                "width": width_name(item.width),
                "value": value,
            });
            monitor.emit(text, Some(payload))?;
        }
        polls += 1;
        if count > 0 && polls >= count {
            break;
        }
        std::thread::sleep(Duration::from_millis(interval_ms as u64));
    }
    Ok(polls)
}

/// Attach to RTT and list the available up channels.
pub fn rtt_info(
    session: &mut SessionManager,
    elf: Option<&Path>,
    address: Option<u64>,
) -> Result<Value, CliError> {
    let address = resolve_control_address(elf, address, "_SEGGER_RTT")?;
    let channels = session.backend().attach_rtt(address)?;
    Ok(json!({ "channels": channels, "address": address }))
}

/// Resolve an explicit address, else the ELF symbol, else `None` (RAM scan).
fn resolve_control_address(
    elf: Option<&Path>,
    address: Option<u64>,
    symbol: &str,
) -> Result<Option<u64>, CliError> {
    if address.is_some() {
        return Ok(address);
    }
    symbols::resolve_from_elf(elf, symbol)
}

fn parse_channels(raw: &str) -> Result<Vec<usize>, CliError> {
    let mut channels = Vec::new();
    for part in raw.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err(CliError::InvalidArgument(format!(
                "invalid channel list '{raw}'"
            )));
        }
        let channel = part
            .parse::<usize>()
            .map_err(|_| CliError::InvalidArgument(format!("invalid RTT channel '{part}'")))?;
        if !channels.contains(&channel) {
            channels.push(channel);
        }
    }
    Ok(channels)
}

/// Poll RTT up channels until `count` polls (0 = until Ctrl-C).
pub fn rtt_monitor(
    session: &mut SessionManager,
    args: &RttMonitorArgs,
    elf: Option<&Path>,
    json: bool,
    stdout: &mut dyn Write,
) -> Result<u32, CliError> {
    let channels = parse_channels(&args.channel)?;
    if channels.is_empty() {
        return Err(CliError::InvalidArgument(
            "rtt monitor needs at least one channel".into(),
        ));
    }
    let address = resolve_control_address(elf, args.address, "_SEGGER_RTT")?;
    let infos = session.backend().attach_rtt(address)?;
    for channel in &channels {
        if !infos.iter().any(|i| i.number == *channel) {
            let available: Vec<String> = infos.iter().map(|i| i.number.to_string()).collect();
            return Err(CliError::InvalidArgument(format!(
                "requested RTT channel {channel} is not present (available: {})",
                if available.is_empty() {
                    "none".into()
                } else {
                    available.join(", ")
                }
            )));
        }
    }
    eprintln!(
        "attached RTT (channels: {})",
        channels
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let opened = LogTarget::from_args(args.log_dir.clone(), args.log_file.clone()).open("rtt")?;
    if let Some((_, path)) = &opened {
        eprintln!("logging to {}", path.display());
    }
    let mut monitor = Monitor::new(json, stdout, opened);
    signal::install();
    signal::reset();
    let mut polls = 0u32;
    loop {
        if signal::interrupted() {
            eprintln!("stopped (Ctrl-C)");
            break;
        }
        let reads = session
            .backend()
            .read_rtt(&channels, args.max_bytes as usize)?;
        let (host_text, host_rfc) = host_now();
        for read in reads {
            let text = String::from_utf8_lossy(&read.data);
            let prefix = match &read.name {
                Some(name) => format!("[RTT{} \"{name}\"]", read.channel),
                None => format!("[RTT{}]", read.channel),
            };
            let line = format!("[{host_text}] {prefix} {text}");
            let payload = json!({
                "host_ts": host_rfc,
                "channel": read.channel,
                "name": read.name,
                "data_hex": hex_encode(&read.data),
                "text": text.to_string(),
            });
            monitor.emit(line, Some(payload))?;
        }
        polls += 1;
        if args.count > 0 && polls >= args.count {
            break;
        }
        std::thread::sleep(Duration::from_millis(args.interval_ms as u64));
    }
    Ok(polls)
}

/// Attach to the Event Recorder and report its state.
pub fn evr_info(
    session: &mut SessionManager,
    elf: Option<&Path>,
    address: Option<u64>,
) -> Result<Value, CliError> {
    let address = resolve_control_address(elf, address, "EventRecorderInfo")?.ok_or_else(|| {
        CliError::InvalidArgument(
            "Event Recorder needs an address: pass --elf with the firmware (EventRecorderInfo symbol) or --address".into(),
        )
    })?;
    let status = session.backend().attach_evr(address)?;
    Ok(json!({ "evr": status }))
}

/// Poll the Event Recorder until `count` polls (0 = until Ctrl-C).
pub fn evr_monitor(
    session: &mut SessionManager,
    args: &EvrMonitorArgs,
    elf: Option<&Path>,
    json: bool,
    stdout: &mut dyn Write,
) -> Result<u32, CliError> {
    let address = resolve_control_address(elf, args.address, "EventRecorderInfo")?.ok_or_else(|| {
        CliError::InvalidArgument(
            "Event Recorder needs an address: pass --elf with the firmware (EventRecorderInfo symbol) or --address".into(),
        )
    })?;
    let status = session.backend().attach_evr(address)?;
    eprintln!(
        "attached Event Recorder (protocol {}, {} records, {} Hz)",
        status.protocol_version, status.record_count, status.ts_freq
    );
    let opened = LogTarget::from_args(args.log_dir.clone(), args.log_file.clone()).open("evr")?;
    if let Some((_, path)) = &opened {
        eprintln!("logging to {}", path.display());
    }
    let mut monitor = Monitor::new(json, stdout, opened);
    signal::install();
    signal::reset();
    let mut polls = 0u32;
    loop {
        if signal::interrupted() {
            eprintln!("stopped (Ctrl-C)");
            break;
        }
        let events = session.backend().read_evr()?;
        let (host_text, host_rfc) = host_now();
        for event in events {
            if !args.ctx.is_empty() && !args.ctx.contains(&(event.context as u32)) {
                continue;
            }
            monitor.emit(
                evr_line(&host_text, &event),
                Some(evr_payload(&host_rfc, &event)),
            )?;
        }
        polls += 1;
        if args.count > 0 && polls >= args.count {
            break;
        }
        std::thread::sleep(Duration::from_millis(args.interval_ms as u64));
    }
    Ok(polls)
}

fn evr_line(host_text: &str, event: &EvrEvent) -> String {
    let mut line = format!(
        "[{host_text}] evr ticks={} t={:.6}s ctx=0x{:X} comp=0x{:02X} msg=0x{:02X} seq={} val1=0x{:08X} val2=0x{:08X}",
        event.timestamp_ticks,
        event.timestamp_secs,
        event.context,
        event.component,
        event.message,
        event.sequence,
        event.val1,
        event.val2,
    );
    if event.first {
        line.push_str(" first");
    }
    if event.last {
        line.push_str(" last");
    }
    if event.irq {
        line.push_str(" irq");
    }
    line
}

fn evr_payload(host_rfc: &str, event: &EvrEvent) -> Value {
    let mut value = serde_json::to_value(event).unwrap_or(Value::Null);
    if let Value::Object(map) = &mut value {
        map.insert("host_ts".into(), Value::String(host_rfc.to_string()));
    }
    value
}

/// List symbols (optionally filtered by a case-insensitive substring).
pub fn symbols_list(elf: Option<&Path>, pattern: Option<&str>) -> Result<Value, CliError> {
    let path =
        elf.ok_or_else(|| CliError::InvalidArgument("provide a firmware ELF with --elf".into()))?;
    let symbols = symbols::load_symbols(path)?;
    let needle = pattern.map(|p| p.to_ascii_lowercase());
    let mut out = Vec::new();
    for (name, address) in &symbols {
        if let Some(needle) = &needle {
            if !name.to_ascii_lowercase().contains(needle) {
                continue;
            }
        }
        out.push(json!({ "name": name, "address": address }));
    }
    Ok(json!({ "count": out.len(), "symbols": out }))
}

/// Resolve a single symbol to its address.
pub fn symbols_resolve(elf: Option<&Path>, name: &str) -> Result<Value, CliError> {
    let path =
        elf.ok_or_else(|| CliError::InvalidArgument("provide a firmware ELF with --elf".into()))?;
    let symbols = symbols::load_symbols(path)?;
    match symbols.get(name) {
        Some(address) => Ok(json!({ "name": name, "address": address, "found": true })),
        None => Ok(json!({ "name": name, "address": Value::Null, "found": false })),
    }
}

/// Options parsed from a REPL `watch run` line.
#[derive(Debug, Clone)]
pub struct WatchRunOpts {
    pub count: u32,
    pub log_dir: Option<PathBuf>,
    pub log_file: Option<PathBuf>,
}

/// Parse `watch run` arguments: `[--count N] [--log-dir D|--log-file F]`.
pub fn parse_watch_run(tokens: &[&str]) -> Result<WatchRunOpts, CliError> {
    let mut opts = WatchRunOpts {
        count: 0,
        log_dir: None,
        log_file: None,
    };
    let mut iter = tokens.iter().peekable();
    while let Some(token) = iter.next() {
        match *token {
            "--count" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--count needs a value".into()))?;
                opts.count = parse_u32_repl(value)?;
            }
            "--log-dir" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--log-dir needs a value".into()))?;
                opts.log_dir = Some(PathBuf::from(value));
            }
            "--log-file" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--log-file needs a value".into()))?;
                opts.log_file = Some(PathBuf::from(value));
            }
            other => {
                return Err(CliError::InvalidArgument(format!(
                    "unknown watch run option '{other}'"
                )))
            }
        }
    }
    if opts.log_dir.is_some() && opts.log_file.is_some() {
        return Err(CliError::InvalidArgument(
            "--log-dir and --log-file are mutually exclusive".into(),
        ));
    }
    Ok(opts)
}

/// Parse `watch add <target> [--width W] [--label L]` into
/// `(target, width, label)`.
pub fn parse_watch_add(tokens: &[&str]) -> Result<(String, String, Option<String>), CliError> {
    let mut target = None;
    let mut width = "u32".to_string();
    let mut label = None;
    let mut iter = tokens.iter().peekable();
    while let Some(token) = iter.next() {
        match *token {
            "--width" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--width needs a value".into()))?;
                width = value.to_string();
            }
            "--label" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--label needs a value".into()))?;
                label = Some(value.to_string());
            }
            other if other.starts_with("--") => {
                return Err(CliError::InvalidArgument(format!(
                    "unknown watch add option '{other}'"
                )))
            }
            other => {
                if target.is_some() {
                    return Err(CliError::InvalidArgument(format!(
                        "unexpected extra argument '{other}'"
                    )));
                }
                target = Some(other.to_string());
            }
        }
    }
    let target = target.ok_or_else(|| {
        CliError::InvalidArgument("watch add needs a target (symbol name or 0xADDR)".into())
    })?;
    Ok((target, width, label))
}

pub fn parse_u32_repl(s: &str) -> Result<u32, CliError> {
    let value = parse_u64_arg(s).map_err(CliError::InvalidArgument)?;
    u32::try_from(value).map_err(|_| CliError::InvalidArgument(format!("number out of range: {s}")))
}

/// Parse REPL `rtt [info] [monitor] [options]` (the `rtt` token is stripped).
pub fn parse_rtt_repl(tokens: &[&str]) -> Result<RttMonitorArgs, CliError> {
    let mut args = RttMonitorArgs {
        channel: "0".into(),
        interval_ms: 200,
        count: 0,
        address: None,
        max_bytes: 1024,
        log_dir: None,
        log_file: None,
    };
    let mut iter = tokens.iter().peekable();
    while let Some(token) = iter.next() {
        match *token {
            "monitor" | "info" => {}
            "--channel" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--channel needs a value".into()))?;
                args.channel = value.to_string();
            }
            "--interval-ms" => {
                let value = iter.next().ok_or_else(|| {
                    CliError::InvalidArgument("--interval-ms needs a value".into())
                })?;
                args.interval_ms = parse_u32_repl(value)?;
            }
            "--count" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--count needs a value".into()))?;
                args.count = parse_u32_repl(value)?;
            }
            "--address" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--address needs a value".into()))?;
                args.address = Some(parse_u64_arg(value).map_err(CliError::InvalidArgument)?);
            }
            "--max-bytes" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--max-bytes needs a value".into()))?;
                args.max_bytes = parse_u32_repl(value)?;
            }
            "--log-dir" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--log-dir needs a value".into()))?;
                args.log_dir = Some(PathBuf::from(value));
            }
            "--log-file" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--log-file needs a value".into()))?;
                args.log_file = Some(PathBuf::from(value));
            }
            other => {
                return Err(CliError::InvalidArgument(format!(
                    "unknown rtt option '{other}'"
                )))
            }
        }
    }
    if args.log_dir.is_some() && args.log_file.is_some() {
        return Err(CliError::InvalidArgument(
            "--log-dir and --log-file are mutually exclusive".into(),
        ));
    }
    Ok(args)
}

/// Parse REPL `evr [info] [monitor] [options]` (the `evr` token is stripped).
pub fn parse_evr_repl(tokens: &[&str]) -> Result<EvrMonitorArgs, CliError> {
    let mut args = EvrMonitorArgs {
        interval_ms: 200,
        count: 0,
        ctx: Vec::new(),
        address: None,
        log_dir: None,
        log_file: None,
    };
    let mut iter = tokens.iter().peekable();
    while let Some(token) = iter.next() {
        match *token {
            "monitor" | "info" => {}
            "--interval-ms" => {
                let value = iter.next().ok_or_else(|| {
                    CliError::InvalidArgument("--interval-ms needs a value".into())
                })?;
                args.interval_ms = parse_u32_repl(value)?;
            }
            "--count" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--count needs a value".into()))?;
                args.count = parse_u32_repl(value)?;
            }
            "--ctx" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--ctx needs a value".into()))?;
                for part in value.split(',') {
                    let part = part.trim();
                    let ctx = part.parse::<u8>().map_err(|_| {
                        CliError::InvalidArgument(format!("invalid evr context '{part}' (0..7)"))
                    })?;
                    if ctx > 7 {
                        return Err(CliError::InvalidArgument(format!(
                            "evr context must be 0..7, got {part}"
                        )));
                    }
                    if !args.ctx.contains(&(ctx as u32)) {
                        args.ctx.push(ctx as u32);
                    }
                }
            }
            "--address" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--address needs a value".into()))?;
                args.address = Some(parse_u64_arg(value).map_err(CliError::InvalidArgument)?);
            }
            "--log-dir" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--log-dir needs a value".into()))?;
                args.log_dir = Some(PathBuf::from(value));
            }
            "--log-file" => {
                let value = iter
                    .next()
                    .ok_or_else(|| CliError::InvalidArgument("--log-file needs a value".into()))?;
                args.log_file = Some(PathBuf::from(value));
            }
            other => {
                return Err(CliError::InvalidArgument(format!(
                    "unknown evr option '{other}'"
                )))
            }
        }
    }
    if args.log_dir.is_some() && args.log_file.is_some() {
        return Err(CliError::InvalidArgument(
            "--log-dir and --log-file are mutually exclusive".into(),
        ));
    }
    Ok(args)
}
