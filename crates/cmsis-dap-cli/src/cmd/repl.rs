use super::{output, parse_protocol, CliError, ReplOptions};
use cmsis_dap_core::error::ErrorCode;
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
  loadbin <file> <addr>      program a binary file (destructive)
  loadfile <file> [<addr>]   program axf/elf/bin/hex (destructive)
  erase                      erase all flash (destructive)
  flash erase_sector <addr> <size>  erase a flash range (destructive)
  verifybin <file> [<addr>]  verify a binary file against memory
  echo <text> | sleep <ms>   misc helpers
  ? | help                   show this help
  q | exit                   quit
Destructive commands require --yes at startup, or interactive approval.";

/// Run the interactive REPL.
///
/// `reader` is the command source and `interactive` toggles the prompt and
/// interactive destructive-approval behavior (kept out of stdin plumbing so
/// tests can drive the REPL with an in-memory reader).
pub fn run(
    opts: &ReplOptions,
    session: &mut SessionManager,
    reader: &mut dyn BufRead,
    interactive: bool,
) -> Result<(), CliError> {
    let mut engine = ScriptEngine::with_connection(
        SecurityPolicy {
            allow_destructive: opts.yes,
        },
        opts.probe_id.clone(),
        parse_protocol(&opts.protocol)?,
        opts.speed_khz,
        opts.target.clone(),
        opts.under_reset,
    );
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
            Err(e) if e.code == ErrorCode::DestructiveDisabled => {
                if interactive && prompt_enable_destructive(&mut *reader)? {
                    engine.policy_mut().allow_destructive = true;
                    match engine.execute_line(session, &line) {
                        Ok(Some(output)) => output::print_result(opts.json, &output),
                        Ok(None) => {}
                        Err(e2) => eprintln!("error: {e2}"),
                    }
                } else {
                    eprintln!("error: {e}");
                }
            }
            Err(e) => eprintln!("error: {e}"),
        }
    }
    Ok(())
}

/// Ask for interactive approval of destructive mode.
///
/// Reads from the same `reader` that drives the REPL. Reading from a fresh
/// `stdin()` handle here would deadlock, because the REPL already holds the
/// stdin lock for its own line reads.
fn prompt_enable_destructive(reader: &mut dyn BufRead) -> Result<bool, CliError> {
    eprint!("This command is destructive (erase/flash). Enable destructive mode for the session? [y/N] ");
    std::io::stderr()
        .flush()
        .map_err(|e| CliError::Aborted(e.to_string()))?;
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| CliError::Aborted(e.to_string()))?;
    Ok(line.trim().eq_ignore_ascii_case("y"))
}
