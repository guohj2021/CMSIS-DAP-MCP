//! Linear debug script engine with a J-Link Commander / OpenOCD style subset.
//!
//! Scripts are executed one command per line (semicolons are also accepted as
//! separators). Comments start with `//` or `#`. Quoted arguments are
//! supported. Destructive commands (erase, loadbin, loadfile, flash
//! write_image, flash erase_sector) additionally require
//! `--allow-destructive`.

use crate::backend::{
    AccessWidth, ConnectOptions, CoreRegister, ExportFormat, ImageFileFormat, Protocol, ResetMode,
};
use crate::error::{ErrorCode, McpError};
use crate::security::{SecurityLevel, SecurityPolicy};
use crate::session::SessionManager;
use serde::Serialize;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct CommandResult {
    pub command: String,
    pub status: String,
    pub output: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScriptReport {
    pub ok: bool,
    pub commands: usize,
    pub results: Vec<CommandResult>,
}

struct Context {
    protocol: Protocol,
    speed_khz: Option<u32>,
    target: Option<String>,
    probe_id: Option<String>,
    under_reset: bool,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            protocol: Protocol::Swd,
            speed_khz: None,
            target: None,
            probe_id: None,
            under_reset: false,
        }
    }
}

/// Reusable per-session script interpreter.
///
/// Keeps connection context (protocol, speed, probe, target) and the security
/// policy across lines, so an interactive REPL can stay connected while the
/// batch `run` wrapper keeps the historical one-shot semantics.
pub struct ScriptEngine {
    ctx: Context,
    policy: SecurityPolicy,
}

impl ScriptEngine {
    pub fn new(policy: SecurityPolicy) -> Self {
        Self {
            ctx: Context::default(),
            policy,
        }
    }

    /// Seed the engine with CLI-provided connection options so the `connect`
    /// command inside scripts/REPL uses them (instead of defaults).
    pub fn with_connection(
        policy: SecurityPolicy,
        probe_id: Option<String>,
        protocol: Protocol,
        speed_khz: Option<u32>,
        target: Option<String>,
        under_reset: bool,
    ) -> Self {
        Self {
            ctx: Context {
                protocol,
                speed_khz,
                target,
                probe_id,
                under_reset,
            },
            policy,
        }
    }

    /// Mutable access to the security policy (used by REPLs to enable
    /// destructive mode interactively).
    pub fn policy_mut(&mut self) -> &mut SecurityPolicy {
        &mut self.policy
    }

    /// Execute a full script and produce the same report shape as `run`.
    pub fn run_script(
        &mut self,
        session: &mut SessionManager,
        script: &str,
    ) -> Result<ScriptReport, McpError> {
        let mut report = ScriptReport {
            ok: true,
            commands: 0,
            results: Vec::new(),
        };

        for line in logical_lines(script) {
            let Some(tokens) = tokenize(&line) else {
                continue;
            };
            if tokens.is_empty() {
                continue;
            }
            report.commands += 1;
            let display = tokens.join(" ");
            match self.execute_line(session, &line) {
                Ok(Some(output)) => {
                    let stopped = output
                        .get("stopped")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    report.results.push(CommandResult {
                        command: display,
                        status: "ok".into(),
                        output,
                    });
                    if stopped {
                        break;
                    }
                }
                Ok(None) => {
                    report.results.push(CommandResult {
                        command: display,
                        status: "ok".into(),
                        output: serde_json::json!(null),
                    });
                }
                Err(e) => {
                    report.results.push(CommandResult {
                        command: display,
                        status: "error".into(),
                        output: serde_json::json!({
                            "code": format!("{:?}", e.code),
                            "message": e.message,
                        }),
                    });
                    report.ok = false;
                    break;
                }
            }
        }

        Ok(report)
    }

    /// Execute one logical line against the session.
    ///
    /// Returns `Ok(None)` for blank or comment-only lines, `Ok(Some(value))`
    /// for executed commands, and `{"stopped": true}` for `q`/`exit`.
    pub fn execute_line(
        &mut self,
        session: &mut SessionManager,
        line: &str,
    ) -> Result<Option<serde_json::Value>, McpError> {
        let Some(tokens) = tokenize(line) else {
            return Ok(None);
        };
        if tokens.is_empty() {
            return Ok(None);
        }
        let name = tokens[0].to_ascii_lowercase();
        let args = &tokens[1..];
        if matches!(name.as_str(), "q" | "exit") {
            return Ok(Some(serde_json::json!({ "stopped": true })));
        }
        let output = dispatch(session, &self.policy, &mut self.ctx, &name, args)?;
        Ok(Some(output))
    }
}

/// Execute a script against the given session under the given security policy.
pub fn run(
    session: &mut SessionManager,
    policy: &SecurityPolicy,
    script: &str,
) -> Result<ScriptReport, McpError> {
    ScriptEngine::new(policy.clone()).run_script(session, script)
}

fn logical_lines(script: &str) -> Vec<String> {
    script.split('\n').flat_map(split_semicolons).collect()
}

fn split_semicolons(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in line.chars() {
        match quote {
            Some(q) => {
                current.push(ch);
                if ch == q {
                    quote = None;
                }
            }
            None => match ch {
                '"' | '\'' => {
                    quote = Some(ch);
                    current.push(ch);
                }
                ';' => {
                    out.push(std::mem::take(&mut current));
                }
                _ => current.push(ch),
            },
        }
    }
    out.push(current);
    out
}

fn tokenize(line: &str) -> Option<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut comment = false;
    let mut prev: Option<char> = None;

    for ch in line.chars() {
        if comment {
            break;
        }
        match quote {
            Some(q) => {
                current.push(ch);
                if ch == q {
                    quote = None;
                }
            }
            None => match ch {
                '"' | '\'' => {
                    quote = Some(ch);
                    current.push(ch);
                }
                '#' => comment = true,
                '/' if prev == Some('/') => {
                    current.pop();
                    comment = true;
                }
                c if c.is_whitespace() => {
                    if !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                    }
                }
                c => current.push(c),
            },
        }
        prev = Some(ch);
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    if tokens.is_empty() {
        None
    } else {
        Some(tokens)
    }
}

fn unquote(token: &str) -> String {
    let t = token.trim();
    if t.len() >= 2
        && ((t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')))
    {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

fn parse_u64(token: &str) -> Result<u64, McpError> {
    let t = unquote(token);
    let (radix, digits) = if let Some(rest) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X"))
    {
        (16, rest)
    } else {
        (10, t.as_str())
    };
    u64::from_str_radix(digits, radix).map_err(|_| {
        McpError::new(
            ErrorCode::InvalidArgument,
            format!("expected a number, got {token}"),
        )
    })
}

fn require_destructive(policy: &SecurityPolicy) -> Result<(), McpError> {
    policy.check(SecurityLevel::Destructive)
}

fn do_loadfile(
    session: &mut SessionManager,
    policy: &SecurityPolicy,
    args: &[String],
) -> Result<serde_json::Value, McpError> {
    require_destructive(policy)?;
    session.require_flash_defined()?;
    let path = args
        .first()
        .ok_or_else(|| McpError::new(ErrorCode::InvalidArgument, "loadfile requires a path"))?;
    let path_str = unquote(path);
    let format = ImageFileFormat::from_extension(Path::new(&path_str)).ok_or_else(|| {
        McpError::new(
            ErrorCode::InvalidArgument,
            "cannot infer file format from extension",
        )
    })?;
    let address = args.get(1).map(|a| parse_u64(a)).transpose()?.unwrap_or(0);
    let bytes = session
        .backend()
        .program_file(Path::new(&path_str), format, address, false)?;
    Ok(serde_json::json!({
        "programmed": true,
        "path": path_str,
        "format": format.as_str(),
        "bytes": bytes,
    }))
}

fn do_erase_sector(
    session: &mut SessionManager,
    policy: &SecurityPolicy,
    args: &[String],
) -> Result<serde_json::Value, McpError> {
    require_destructive(policy)?;
    session.require_flash_defined()?;
    let address = args
        .first()
        .map(|a| parse_u64(a))
        .transpose()?
        .ok_or_else(|| {
            McpError::new(
                ErrorCode::InvalidArgument,
                "flash erase_sector requires an address",
            )
        })?;
    let size = args
        .get(1)
        .map(|s| parse_u64(s))
        .transpose()?
        .ok_or_else(|| {
            McpError::new(
                ErrorCode::InvalidArgument,
                "flash erase_sector requires a size",
            )
        })?;
    session.backend().erase_flash(address, size)?;
    Ok(serde_json::json!({ "erased": true, "address": address, "size": size }))
}

fn dispatch(
    session: &mut SessionManager,
    policy: &SecurityPolicy,
    ctx: &mut Context,
    name: &str,
    args: &[String],
) -> Result<serde_json::Value, McpError> {
    let invalid = |msg: &str| McpError::new(ErrorCode::InvalidArgument, msg);
    match name {
        // ---- session ----
        "connect" | "init" => {
            let opts = ConnectOptions {
                probe_id: ctx.probe_id.clone(),
                protocol: ctx.protocol,
                speed_khz: ctx.speed_khz,
                target: ctx.target.clone(),
                under_reset: ctx.under_reset,
            };
            let info = session.connect(&opts)?;
            Ok(serde_json::json!({ "target": info }))
        }
        "disconnect" => {
            session.disconnect()?;
            Ok(serde_json::json!({ "disconnected": true }))
        }
        "si" => {
            let value = args
                .first()
                .ok_or_else(|| invalid("si requires an interface"))?;
            ctx.protocol = match value.to_ascii_lowercase().as_str() {
                "swd" => Protocol::Swd,
                "jtag" => Protocol::Jtag,
                other => {
                    return Err(invalid(&format!(
                        "interface must be swd or jtag, got {other}"
                    )))
                }
            };
            let interface = if ctx.protocol == Protocol::Swd {
                "swd"
            } else {
                "jtag"
            };
            Ok(serde_json::json!({ "interface": interface }))
        }
        "speed" | "adapter speed" => {
            let value = args
                .first()
                .ok_or_else(|| invalid("speed requires a value in kHz"))?;
            ctx.speed_khz = Some(parse_u64(value)? as u32);
            Ok(serde_json::json!({ "speed_khz": ctx.speed_khz }))
        }
        "device" => {
            let value = args.join(" ");
            if value.is_empty() {
                return Err(invalid("device requires a name"));
            }
            ctx.target = Some(value.clone());
            Ok(serde_json::json!({ "target": value }))
        }
        "adapter" => {
            let Some(serial) = args.first() else {
                return Err(invalid("adapter serial requires a value"));
            };
            if !serial.eq_ignore_ascii_case("serial") {
                return Err(invalid("unknown adapter command"));
            }
            let value = args
                .get(1)
                .ok_or_else(|| invalid("adapter serial requires a value"))?;
            ctx.probe_id = Some(unquote(value));
            Ok(serde_json::json!({ "serial": ctx.probe_id }))
        }
        "targets" => {
            let info = session.target_info();
            Ok(serde_json::json!({
                "targets": info.map(|t| serde_json::json!({ "core_type": t.core_type })).into_iter().collect::<Vec<_>>()
            }))
        }
        "cmsis-dap" => {
            let Some(sub) = args.first() else {
                return Err(invalid("cmsis-dap requires vid_pid"));
            };
            if !sub.eq_ignore_ascii_case("vid_pid") {
                return Err(invalid("unknown cmsis-dap command"));
            }
            Ok(serde_json::json!({ "accepted": true, "note": "ignored" }))
        }

        // ---- core ----
        "halt" => {
            session.backend().halt()?;
            Ok(serde_json::json!({ "halted": true }))
        }
        "go" | "resume" => {
            session.backend().resume()?;
            Ok(serde_json::json!({ "running": true }))
        }
        "step" => {
            session.backend().step()?;
            Ok(serde_json::json!({ "stepped": true }))
        }
        "reset" => {
            let mode = match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
                None | Some("run") => ResetMode::Run,
                Some("halt") => ResetMode::Halt,
                Some(other) => {
                    return Err(invalid(&format!(
                        "reset mode must be run or halt, got {other}"
                    )))
                }
            };
            session.backend().reset(mode)?;
            Ok(serde_json::json!({
                "reset": true,
                "mode": if mode == ResetMode::Run { "run" } else { "halt" },
            }))
        }
        "reg" => {
            let name = args
                .first()
                .ok_or_else(|| invalid("reg requires a register name"))?;
            let reg = CoreRegister::Name(unquote(name));
            if let Some(value) = args.get(1) {
                let value = parse_u64(value)?;
                session.backend().write_core_register(&reg, value)?;
                Ok(
                    serde_json::json!({ "register": unquote(name), "value": value, "written": true }),
                )
            } else {
                let value = session.backend().read_core_register(&reg)?;
                Ok(serde_json::json!({ "register": unquote(name), "value": value }))
            }
        }
        "regs" => {
            let registers = session.backend().list_core_registers()?;
            Ok(serde_json::json!({ "registers": registers }))
        }

        // ---- memory ----
        "mem8" | "mdb" => memory_read(session, name, AccessWidth::U8, args),
        "mem16" | "mdh" => memory_read(session, name, AccessWidth::U16, args),
        "mem32" | "mdw" => memory_read(session, name, AccessWidth::U32, args),
        "w8" | "mwb" => memory_write(session, policy, AccessWidth::U8, args),
        "w16" | "mwh" => memory_write(session, policy, AccessWidth::U16, args),
        "w32" | "mww" => memory_write(session, policy, AccessWidth::U32, args),

        // ---- files / flash ----
        "savebin" | "dump_image" => {
            let path = args
                .first()
                .ok_or_else(|| invalid("savebin requires a path"))?;
            let address = args.get(1).map(|a| parse_u64(a)).transpose()?.unwrap_or(0);
            let size = args
                .get(2)
                .map(|s| parse_u64(s))
                .transpose()?
                .ok_or_else(|| invalid("savebin requires a size"))?;
            let bytes = session.backend().export_memory(
                Path::new(&unquote(path)),
                ExportFormat::Bin,
                address,
                size,
            )?;
            Ok(serde_json::json!({
                "exported": true,
                "path": unquote(path),
                "format": "bin",
                "bytes": bytes,
            }))
        }
        "loadbin" => {
            require_destructive(policy)?;
            session.require_flash_defined()?;
            let path = args
                .first()
                .ok_or_else(|| invalid("loadbin requires a path"))?;
            let address = args
                .get(1)
                .map(|a| parse_u64(a))
                .transpose()?
                .ok_or_else(|| invalid("loadbin requires an address"))?;
            let bytes = session.backend().program_file(
                Path::new(&unquote(path)),
                ImageFileFormat::Bin,
                address,
                false,
            )?;
            Ok(serde_json::json!({
                "programmed": true,
                "path": unquote(path),
                "format": "bin",
                "bytes": bytes,
            }))
        }
        "loadfile" => do_loadfile(session, policy, args),
        "flash" => {
            let Some(sub) = args.first() else {
                return Err(invalid("flash requires erase_sector or write_image"));
            };
            match sub.to_ascii_lowercase().as_str() {
                "erase_sector" => do_erase_sector(session, policy, &args[1..]),
                "write_image" => do_loadfile(session, policy, &args[1..]),
                _ => Err(invalid("flash command must be erase_sector or write_image")),
            }
        }
        "erase" => {
            require_destructive(policy)?;
            session.require_flash_defined()?;
            session.backend().erase_flash(0, u64::MAX)?;
            Ok(serde_json::json!({ "erased": true }))
        }
        "verifybin" | "verify_image" => {
            let path = args
                .first()
                .ok_or_else(|| invalid("verifybin requires a path"))?;
            let address = args.get(1).map(|a| parse_u64(a)).transpose()?.unwrap_or(0);
            let data = std::fs::read(Path::new(&unquote(path)))
                .map_err(|e| McpError::new(ErrorCode::FileError, e.to_string()))?;
            let values =
                session
                    .backend()
                    .read_memory(address, AccessWidth::U8, data.len() as u32)?;
            let mismatches = data
                .iter()
                .zip(values.iter())
                .filter(|(expected, actual)| **expected != **actual as u8)
                .count();
            Ok(serde_json::json!({
                "verified": mismatches == 0,
                "mismatches": mismatches,
            }))
        }

        // ---- misc ----
        "sleep" => {
            let ms = args
                .first()
                .map(|a| parse_u64(a))
                .transpose()?
                .ok_or_else(|| invalid("sleep requires milliseconds"))?;
            std::thread::sleep(Duration::from_millis(ms));
            Ok(serde_json::json!({ "slept_ms": ms }))
        }
        "echo" => Ok(serde_json::json!({ "text": args.join(" ") })),

        other => Err(invalid(&format!("unknown script command {other}"))),
    }
}

fn memory_read(
    session: &mut SessionManager,
    name: &str,
    width: AccessWidth,
    args: &[String],
) -> Result<serde_json::Value, McpError> {
    let address = args
        .first()
        .map(|a| parse_u64(a))
        .transpose()?
        .ok_or_else(|| {
            McpError::new(
                ErrorCode::InvalidArgument,
                format!("{name} requires an address"),
            )
        })?;
    let count = args.get(1).map(|c| parse_u64(c)).transpose()?.unwrap_or(1) as u32;
    let values = session.backend().read_memory(address, width, count)?;
    Ok(serde_json::json!({
        "address": address,
        "width": match width {
            AccessWidth::U8 => "u8",
            AccessWidth::U16 => "u16",
            AccessWidth::U32 => "u32",
            AccessWidth::U64 => "u64",
        },
        "values": values,
    }))
}

/// Convert a u64 value to a little-endian byte vector matching the access width.
fn value_to_bytes(value: u64, width: AccessWidth) -> Vec<u8> {
    match width {
        AccessWidth::U8 => vec![value as u8],
        AccessWidth::U16 => (value as u16).to_le_bytes().to_vec(),
        AccessWidth::U32 => (value as u32).to_le_bytes().to_vec(),
        AccessWidth::U64 => value.to_le_bytes().to_vec(),
    }
}

fn memory_write(
    session: &mut SessionManager,
    policy: &SecurityPolicy,
    width: AccessWidth,
    args: &[String],
) -> Result<serde_json::Value, McpError> {
    let invalid = |msg: &str| McpError::new(ErrorCode::InvalidArgument, msg);
    let address = args
        .first()
        .map(|a| parse_u64(a))
        .transpose()?
        .ok_or_else(|| invalid("write requires an address"))?;
    let value = args
        .get(1)
        .map(|v| parse_u64(v))
        .transpose()?
        .ok_or_else(|| invalid("write requires a value"))?;
    // When the target address falls inside a flash (NVM) region, route
    // through the flash algorithm instead of a raw memory write, because
    // direct writes to flash are ignored by the flash controller. Writing
    // flash is destructive (it runs the flash algorithm, which erases the
    // affected sector and resets/halts the core), so it is gated behind
    // the destructive policy like `erase` and `loadbin`.
    let in_flash = session.target_info().is_some_and(|info| {
        let end = address.saturating_add(width.byte_size());
        info.memory_regions
            .iter()
            .any(|r| r.kind == "nvm" && address >= r.start && end <= r.end)
    });
    if in_flash {
        require_destructive(policy)?;
        let bytes = value_to_bytes(value, width);
        // Preserve unwritten bytes in the erased sector so a word/byte
        // write only changes the target bytes instead of wiping siblings.
        session
            .backend()
            .program_flash_keep_unwritten(address, &bytes, false)?;
    } else {
        session.backend().write_memory(address, width, &[value])?;
    }
    Ok(serde_json::json!({
        "address": address,
        "value": value,
        "written": true,
        "flash_programmed": in_flash,
    }))
}
