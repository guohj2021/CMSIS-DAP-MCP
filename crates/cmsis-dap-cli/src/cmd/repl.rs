use super::{live, output, parse_protocol, CliError, ReplOptions};
use cmsis_dap_core::backend::AccessWidth;
use cmsis_dap_core::script::ScriptEngine;
use cmsis_dap_core::security::SecurityPolicy;
use cmsis_dap_core::session::SessionManager;
use std::io::{BufRead, Write};

const HELP: &str = "\
J-Link Commander / OpenOCD style commands:
  connect | disconnect        manage the debug session
  si swd|jtag                select interface (default swd)
  speed <khz>                set SWD/JTAG clock speed
  device <name>              select target chip
  adapter serial <id>        select probe by id/serial
  halt | go | step           control execution
  reset [run|halt]           reset the target
  reg <name> [<value>]       read or write a core register
  regs                       list core registers
  mem8/16/32 <addr> [<n>]    read memory
  w8/16/32 <addr> <value>    write memory
  savebin <file> <addr> <size>   export memory to a binary file
  loadbin <file> <addr>      program a binary file (erases/writes flash)
  loadfile <file> [<addr>]   program axf/elf/bin/hex (erases/writes flash)
  erase                      erase all flash
  flash erase_sector <addr> <size>  erase a flash range
  verifybin <file> [<addr>]  verify a binary file against memory
  watch add <name|addr> [--width u8|u16|u32|u64] [--label TEXT]
  watch list | remove <idx|name> | clear | interval <ms>
  watch run [--count N] [--log-dir D | --log-file F]   live variable watch
  rtt [info] [--channel 0,1] [--count N] [--interval-ms N] [--log-dir D | --log-file F]
  evr [info] [--ctx 0..7] [--count N] [--log-dir D | --log-file F]
  echo <text> | sleep <ms>   misc helpers
  ? | help                   show this help
  q | exit                   quit
Flash erase/program run directly; they still require a target that defines
flash (set device/--target, then connect).
Watch/RTT/EV Recorder need a session; provide --elf for symbol names.";

/// Run the interactive REPL.
///
/// `reader` is the command source and `interactive` toggles the prompt (kept
/// out of stdin plumbing so tests can drive the REPL with an in-memory reader).
pub fn run(
    opts: &ReplOptions,
    session: &mut SessionManager,
    reader: &mut dyn BufRead,
    interactive: bool,
) -> Result<(), CliError> {
    let mut engine = ScriptEngine::with_connection(
        SecurityPolicy {
            allow_destructive: true,
        },
        opts.probe_id.clone(),
        parse_protocol(&opts.protocol)?,
        opts.speed_khz,
        opts.target.clone(),
        opts.under_reset,
    );
    let mut watch_state = live::WatchState::default();
    loop {
        if interactive {
            eprint!("cmsis-dap-cli> ");
            std::io::stderr()
                .flush()
                .map_err(|e| CliError::Aborted(e.to_string()))?;
        }
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|e| CliError::Aborted(format!("failed to read stdin: {e}")))?;
        if read == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if matches!(trimmed, "q" | "quit" | "exit") {
            break;
        }
        if matches!(trimmed, "?" | "help") {
            eprintln!("{HELP}");
            continue;
        }
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        match tokens.first().copied() {
            Some("watch") => {
                if repl_watch(&tokens[1..], opts, &mut watch_state, session)? {
                    break;
                }
                continue;
            }
            Some("rtt") => {
                if repl_rtt(&tokens[1..], opts, session)? {
                    break;
                }
                continue;
            }
            Some("evr") => {
                if repl_evr(&tokens[1..], opts, session)? {
                    break;
                }
                continue;
            }
            _ => {}
        }
        match engine.execute_line(session, &line) {
            Ok(Some(output)) => {
                if output
                    .get("stopped")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    break;
                }
                output::print_result(opts.json, &output);
            }
            Ok(None) => {}
            Err(e) => eprintln!("error: {e}"),
        }
    }
    Ok(())
}

fn load_elf_symbols(
    opts: &ReplOptions,
) -> Result<Option<std::collections::BTreeMap<String, u64>>, CliError> {
    match opts.elf.as_deref() {
        Some(path) => Ok(Some(super::symbols::load_symbols(path)?)),
        None => Ok(None),
    }
}

/// Handle `watch ...` lines; returns true to exit the REPL.
fn repl_watch(
    tokens: &[&str],
    opts: &ReplOptions,
    state: &mut live::WatchState,
    session: &mut SessionManager,
) -> Result<bool, CliError> {
    match tokens.first().copied() {
        Some("add") => {
            let (target, width, label) = live::parse_watch_add(&tokens[1..])?;
            let width = super::parse_width(&width)?;
            let symbols = load_elf_symbols(opts)?;
            state.add(&target, width, label, symbols.as_ref())?;
            let item = state.items.last().expect("just added");
            println!(
                "watch {}: {} @0x{:X} ({})",
                state.items.len() - 1,
                item.label,
                item.address,
                match width {
                    AccessWidth::U8 => "u8",
                    AccessWidth::U16 => "u16",
                    AccessWidth::U32 => "u32",
                    AccessWidth::U64 => "u64",
                }
            );
        }
        Some("list") => {
            if state.items.is_empty() {
                println!("no watched variables");
            } else {
                for (i, item) in state.items.iter().enumerate() {
                    println!(
                        "{}: {} = 0x{:X} ({})",
                        i,
                        item.label,
                        item.address,
                        match item.width {
                            AccessWidth::U8 => "u8",
                            AccessWidth::U16 => "u16",
                            AccessWidth::U32 => "u32",
                            AccessWidth::U64 => "u64",
                        }
                    );
                }
            }
        }
        Some("remove") => {
            let selector = tokens.get(1).ok_or_else(|| {
                CliError::InvalidArgument("watch remove needs an index or name".into())
            })?;
            state.remove(selector)?;
            println!("removed {selector}");
        }
        Some("clear") => {
            state.clear();
            println!("cleared watch list");
        }
        Some("interval") => {
            let value = tokens.get(1).ok_or_else(|| {
                CliError::InvalidArgument("watch interval needs a value in ms".into())
            })?;
            state.interval_ms = live::parse_u32_repl(value)?;
            println!("watch interval set to {} ms", state.interval_ms);
        }
        Some("run") => {
            let opts_run = live::parse_watch_run(&tokens[1..])?;
            let log = live::LogTarget::from_args(opts_run.log_dir, opts_run.log_file);
            let mut stdout = std::io::stdout();
            live::watch_run(
                session,
                &state.items,
                state.interval_ms,
                opts_run.count,
                opts.json,
                &mut stdout,
                log,
            )?;
        }
        Some(other) => {
            return Err(CliError::InvalidArgument(format!(
                "unknown watch command '{other}' (add|list|remove|clear|interval|run)"
            )))
        }
        None => {
            return Err(CliError::InvalidArgument(
                "watch needs a subcommand (add|list|remove|clear|interval|run)".into(),
            ))
        }
    }
    Ok(false)
}

/// Handle `rtt ...` lines; returns true to exit the REPL.
fn repl_rtt(
    tokens: &[&str],
    opts: &ReplOptions,
    session: &mut SessionManager,
) -> Result<bool, CliError> {
    if tokens.first().copied() == Some("info") {
        let value = live::rtt_info(session, opts.elf.as_deref(), None)?;
        output::print_result(opts.json, &value);
        return Ok(false);
    }
    let args = live::parse_rtt_repl(tokens)?;
    let mut stdout = std::io::stdout();
    live::rtt_monitor(session, &args, opts.elf.as_deref(), opts.json, &mut stdout)?;
    Ok(false)
}

/// Handle `evr ...` lines; returns true to exit the REPL.
fn repl_evr(
    tokens: &[&str],
    opts: &ReplOptions,
    session: &mut SessionManager,
) -> Result<bool, CliError> {
    if tokens.first().copied() == Some("info") {
        let value = live::evr_info(session, opts.elf.as_deref(), None)?;
        output::print_result(opts.json, &value);
        return Ok(false);
    }
    let args = live::parse_evr_repl(tokens)?;
    let mut stdout = std::io::stdout();
    live::evr_monitor(session, &args, opts.elf.as_deref(), opts.json, &mut stdout)?;
    Ok(false)
}
