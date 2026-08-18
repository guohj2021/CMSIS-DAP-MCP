# CLI

## Introduction

`cmsis-dap-cli` is a standalone command-line tool for humans, scripts and
automation. It shares the same `cmsis-dap-core` engine as the MCP server
(probe enumeration, memory, core control, SVD, flash and scripting), but talks
to you directly instead of over MCP.

The repository is a Cargo workspace with three crates:

- `cmsis-dap-core` — the shared engine (backend, session, SVD, script engine);
- `cmsis-dap-mcp` — the MCP server binary;
- `cmsis-dap-cli` — this CLI, which only depends on `cmsis-dap-core`.

## Install

Once the npm package is published with a release, zero-install works via:

```bash
npx -y cmsis-dap-cli --help
```

Until then (or for offline use), download a native binary for Windows / Linux
/ macOS from [GitHub Releases](https://github.com/guohj2021/CMSIS-DAP-MCP/releases),
or build locally:

```bash
cargo build --release --workspace
./target/release/cmsis-dap-cli --help        # target\release\cmsis-dap-cli.exe on Windows
```

To call it as plain `cmsis-dap-cli`, add the directory to `PATH`.

## Quick start

```bash
cmsis-dap-cli list                                   # enumerate probes
cmsis-dap-cli --probe-id 0123456789AB connect        # connect (auto-selects chip)
cmsis-dap-cli read --address 0x20000000 --width u32 --count 4
cmsis-dap-cli halt
cmsis-dap-cli reg get pc
cmsis-dap-cli resume
```

Commands that need a target auto-connect using the global connection options,
so a typical one-shot session looks like:

```text
$ cmsis-dap-cli --target STM32F030C8 connect
target: {"ap_count":1,"core_count":1,"core_type":"Armv6m",...,
         "memory_regions":[FLASH 0x08000000-0x08010000, SRAM 0x20000000-0x20002000]}
```

## Global options

All options are global and can appear before or after the subcommand.

| Option | Meaning |
| --- | --- |
| `--probe-id ID` | probe id or serial to select when several probes are connected |
| `--protocol swd\|jtag` | debug wire protocol (default `swd`) |
| `--speed-khz N` | SWD/JTAG clock speed in kHz |
| `--target NAME` | target chip name (probe-rs built-in or a variant from `--target-yaml`) |
| `--under-reset` | connect while holding reset (locked / unresponsive targets) |
| `--target-yaml FILE` | load a target YAML (chip + flash algorithm definitions) |
| `--svd FILE` | SVD file for named peripheral access (`svd` subcommands) |
| `--elf FILE` | firmware ELF for symbol resolution (`symbols`, `watch`, `rtt`, `evr`) |
| `--json` | machine-readable JSON output instead of human text |
| `--log-level LEVEL` | tracing filter; logs always go to stderr (default `warn`) |
| `--log-file FILE` | write logs to a file instead of stderr |

Numbers (addresses, sizes, values) accept decimal or hex (`0x...`).

## Command reference

### Probe and session

| Command | Purpose |
| --- | --- |
| `list` | enumerate connected probes |
| `info` | show probe information (id, vendor, product, serial, capabilities) |
| `connect` | connect to the target and show target info |
| `disconnect` | disconnect the session |
| `target` | show target info (auto-connects) |

### Memory

| Command | Purpose |
| --- | --- |
| `read --address A --width W --count N [--output FILE --format bin\|hex]` | read memory, or export a range to a file (then `count` is bytes) |
| `write --address A --width W --values V1,V2,...` | write memory |
| `verify --address A --width W --values ...` | compare memory against expected values |

`width` is `u8`, `u16`, `u32` or `u64`.

```bash
cmsis-dap-cli read --address 0x20000000 --width u32 --count 4
cmsis-dap-cli read --address 0x08000000 --width u8 --count 0x1000 --output fw.bin --format bin
cmsis-dap-cli write --address 0x20000000 --width u32 --values 0xDEADBEEF,1,2
```

### Core

| Command | Purpose |
| --- | --- |
| `regs` | list core register names |
| `reg get NAME\|NUM` | read a register (name or number) |
| `reg set NAME\|NUM VALUE` | write a register |
| `status` | show core state, halt reason and PC |
| `halt` / `resume` / `step` | control execution |
| `reset [--mode run\|halt]` | reset and continue, or reset and halt |

Register reads on a running core fail — halt first (each one-shot command
opens a new session, so use `script`/`repl` to halt and read in one session):

```bash
cmsis-dap-cli script --text "connect\nhalt\nreg pc\nresume"
```

### Breakpoints and watchpoints

```text
bp set ADDR | bp list | bp clear
wp set ADDR --access read|write|rw | wp list | wp clear
```

### DAP

```text
dap read ADDR
dap write ADDR VALUE
```

Raw DP/AP register access (`ADDR` bit 24..31 selects the AP, low bits the
register).

### SVD (named peripheral access)

```text
svd list
svd read PERIPH.REG[.FIELD]
svd write PERIPH.REG[.FIELD] VALUE
```

Requires `--svd FILE`. Target syntax: `GPIOA.ODR` or `GPIOA.ODR.ODR0`. Field
writes are read-modify-write.

```bash
cmsis-dap-cli --svd target.svd svd list
cmsis-dap-cli --svd target.svd svd read GPIOA.ODR.ODR0
cmsis-dap-cli --svd target.svd svd write GPIOA.ODR.ODR0 1
```

### Flash

```text
flash erase --address A --size N
flash program --address A --file FILE [--format elf|axf|bin|hex] [--verify]
```

Flash erase/program run directly (no confirmation). They require a target that
defines flash — otherwise the command fails with a clear error instead of
silently doing nothing. `--format` defaults to the file extension. `--verify`
reads the programmed data back.

```bash
cmsis-dap-cli flash erase --address 0x08000000 --size 0x1000
cmsis-dap-cli flash program --address 0x08000000 --file fw.hex --verify
```

### Scripts

```text
script --file FILE
script --text TEXT
```

Runs a J-Link Commander / OpenOCD style script (see [Scripting](./scripting.md)).
The `script` command inherits the global connection options, so `connect`
inside the script uses them.

### Chip tooling

```text
chip generate --flm FILE --flash-start A --flash-size N --sram-start A --sram-size N [--name NAME] [--output FILE]
chip list
chip search KEYWORD
```

`chip generate` builds a probe-rs target YAML from a Keil FLM (see below).
`chip list` / `chip search` list or search the built-in chip database (plus
`--target-yaml` custom chips); results include flash/RAM ranges so you can tell
at a glance whether a chip can be programmed.

### Symbols

```text
symbols list [PATTERN]
symbols resolve NAME
```

Inspect the symbol table of a firmware ELF passed with `--elf`. `list` prints
every symbol (optionally filtered by a case-insensitive substring) with its
virtual address; `resolve` looks up one name. These are the same symbols used
by `watch`, `rtt` and `evr` to find variables and control blocks.

```bash
cmsis-dap-cli --elf firmware.axf symbols resolve counter
cmsis-dap-cli --elf firmware.axf symbols list counter
```

### Live watch

```text
watch [--interval-ms N] [--count N] [--width u8|u16|u32|u64]
      [--log-dir DIR | --log-file FILE] TARGET...
```

Polls one or more variables and prints a timestamped sample line on every
interval. `TARGET` is a symbol name (resolved via `--elf`) or a `0xADDR`
address. Defaults: `--interval-ms 500`, `--count 1` (one sample), `--width
u32`. `--count 0` runs until Ctrl-C; after a clean Ctrl-C stop the command
exits 0 with `stopped (Ctrl-C)` on stderr.

```bash
cmsis-dap-cli --target STM32F030C8 --elf firmware.axf \
  watch counter 0x20000004 --interval-ms 200 --count 0
```

Example output (verified on a Cortex-M0+ target with a CMSIS-DAP probe):

```text
[2026-08-16 19:16:13.302] watch_var = 0x00001007
[2026-08-16 19:16:13.520] watch_var = 0x0000100E
[2026-08-16 19:16:13.736] watch_var = 0x00001015
```

### RTT (J-Link RTT logs)

```text
rtt info
rtt monitor --channel 0,1 [--interval-ms N] [--count N]
            [--address A] [--log-dir DIR | --log-file FILE]
```

`rtt info` attaches to the target RTT control block and lists the up channels.
`rtt monitor` polls the selected up channels (comma list, default `0`) and
prints every received chunk with a host timestamp and channel prefix
(`[RTT0 "Channel 0"] ...`). The control block address is taken from the
`_SEGGER_RTT` symbol of `--elf`, from `--address`, or found by scanning the
target RAM — scanning needs a chip target that defines RAM (built-in chip or
`--target-yaml`). Defaults: `--interval-ms 200`, `--count 0` (until Ctrl-C),
`--max-bytes 1024` per channel per poll.

The firmware must run SEGGER RTT (for example `rtt_target` or the SEGGER RTT
implementation) and initialize the control block before the host attaches.

```bash
cmsis-dap-cli --target STM32F030C8 --elf firmware.axf \
  rtt monitor --channel 0 --count 0 --log-dir logs
```

### Event Recorder (CMSIS-View)

```text
evr info
evr monitor [--interval-ms N] [--count N]
            [--ctx 0..7] [--address A]
            [--log-dir DIR | --log-file FILE]
```

`evr info` attaches to the on-chip Event Recorder and reports its protocol
version, record count, timestamp frequency and counters. `evr monitor` polls
the circular buffer over plain SWD/JTAG memory reads (no trace hardware, no
UART) and prints every new event, decoded from the official 16-byte record
layout: host timestamp, target tick count and seconds (via `ts_freq`), event
context (record `info` bits 16..18, 0..7), component and message numbers,
sequence and the two 32-bit values. `--ctx` filters by context (repeatable or
comma list). Note that the on-chip record stores a 16-bit event id
(component + message); the API level is used for filtering inside the target
and is not part of the stored record.

The firmware must include the CMSIS-View Event Recorder component (symbol
`EventRecorderInfo`) and initialize it before the host attaches. The info
address comes from the `EventRecorderInfo` symbol of `--elf` or from
`--address`.

```bash
cmsis-dap-cli --target STM32F030C8 --elf firmware.axf \
  evr monitor --ctx 0,2 --count 0 --log-dir logs
```

### Monitor output, timestamps and log export

Every `watch`, `rtt monitor` and `evr monitor` line carries a host capture
timestamp `[YYYY-MM-DD HH:MM:SS.mmm]`. With `--json` each sample/event is one
NDJSON object on stdout with a `host_ts` field (RFC 3339 with milliseconds and
time zone); EVR events keep their target `timestamp_ticks` / `timestamp_secs`.

Monitor output is also written to a log file by default. The location is the
current directory with an auto-generated name (`watch-<unix>.log`,
`rtt-<unix>.log`, `evr-<unix>.log`); `--log-dir DIR` selects another directory
(created if missing) and `--log-file FILE` appends to an exact file instead.
The file contains exactly what stdout prints, one line per sample/event,
flushed immediately. Monitor start prints `logging to <path>` on stderr.

### Interactive shell

```text
repl
```

### Non-invasive debugging

```text
dump [--address A]... [--stack-words N] [--no-restore]
```

Takes a snapshot of the target CPU **without resetting it**: registers,
Cortex-M fault status registers (CFSR/HFSR/DFSR/MMFAR/BFAR, read without
halting), the top words of the MSP/PSP stacks and optional memory samples.
Core registers require a short halt; by default the previous run state is
restored afterwards (`--no-restore` leaves the core halted). `--address`
accepts `0xADDR` or ELF symbol names (via `--elf`).

```bash
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 dump \
  --address 0x20000000 --stack-words 16
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 --elf fw.axf dump \
  --address counter --no-restore --json
```

### Remote TCP server

```text
tcp-server [--port 4000]
```

Serves a line-delimited JSON-RPC protocol over TCP on `127.0.0.1`. Requests
mirror the MCP tool names (`list_probes`, `connect`, `read_memory`,
`write_memory`, `read_core_register`, `halt`, `resume`, `step`, `reset`,
`status`, `dump_cpu_state`, ...), one JSON object per line, with
`{"id":N,"result":...}` / `{"id":N,"error":{...}}` responses. A follow-up
request reuses the same session — no reconnect needed. `cmsis-dap-mcp --tcp
PORT` serves the same protocol alongside the MCP stdio server.

```bash
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 tcp-server --port 4000
echo '{"id":1,"method":"read_memory","params":{"address":536870912,"width":"u32","count":4}}' \
  | nc 127.0.0.1 4000
```

### GDB server

```text
gdb-server [--port 1337] [--reset-halt]
```

Exposes a GDB Remote Serial Protocol stub (ported from
[probe-rs-tools](https://github.com/probe-rs/probe-rs) via
[gdbstub](https://github.com/daniel5151/gdbstub), MIT OR Apache-2.0): connect
any GDB (`target remote :1337`) to read/write registers and memory, run,
single-step, halt and use hardware breakpoints. Attach is non-invasive (no
reset; `--reset-halt` opts in). `cmsis-dap-mcp --gdb-port 1337` starts the
same server inside the MCP process.

```bash
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 gdb-server --port 1337
arm-none-eabi-gdb fw.elf -ex 'target remote :1337' -ex 'info registers'
```

Related references: the GDB Remote Serial Protocol and the MCP specification
([modelcontextprotocol.io](https://modelcontextprotocol.io)); Cortex-M fault
status registers are part of the ARM System Control Block (see the
[ARMv6-M Architecture Reference Manual](https://developer.arm.com/documentation/ddi0419/));
Event Recorder details are in the
[CMSIS-View documentation](https://arm-software.github.io/CMSIS-View/latest/).

## Generating a target YAML from an FLM

For chips that are not built into probe-rs, flashing needs a target YAML that
describes the chip and embeds the vendor flash algorithm. You do not have to
hand-write it — `chip generate` reads a Keil FLM and only needs the Flash and
SRAM address ranges from you:

```bash
cmsis-dap-cli chip generate \
  --flm MyChip_64.FLM \
  --flash-start 0x08000000 --flash-size 0x10000 \
  --sram-start 0x20000000 --sram-size 0x2000 \
  --name MYCHIP --output MYCHIP.yaml
```

Everything else is extracted from the FLM automatically: the algorithm
instructions, entry-point offsets (`Init`/`ProgramPage`/`EraseSector`/
`EraseChip`), the static data base, the FlashDevice descriptor (page size,
erased value, sector size, timeouts) and the device name. `--name` defaults to
the FLM file stem; use `--output -` to print the YAML to stdout.

Then connect with it:

```bash
cmsis-dap-cli --target-yaml MYCHIP.yaml connect
```

When the target YAML defines exactly one chip variant, `--target` can be
omitted — the CLI auto-selects it. With several variants, `--target NAME` is
required (the command lists the available names).

The generated YAML places the algorithm at `SRAM start + 0x20`; make sure the
SRAM range is large enough (the command refuses to emit a YAML that would not
fit).

## Listing and searching chips

```bash
cmsis-dap-cli chip list
cmsis-dap-cli chip search STM32F103
cmsis-dap-cli chip search stm32f103c8
cmsis-dap-cli --target-yaml MYCHIP.yaml chip search MYCHIP
```

Search is case-insensitive and matches substrings. With `--json` the full
details (family, cores, flash and RAM ranges) are returned for scripting.

## Examples

### End-to-end debug session

```bash
cmsis-dap-cli list
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 connect
cmsis-dap-cli --target STM32F030C8 read --address 0x20000000 --width u32 --count 4
cmsis-dap-cli --target STM32F030C8 halt
cmsis-dap-cli --target STM32F030C8 reg get pc
cmsis-dap-cli --target STM32F030C8 step
cmsis-dap-cli --target STM32F030C8 resume
```

### Program firmware and verify

```bash
cmsis-dap-cli --target STM32F030C8 flash erase --address 0x08000000 --size 0x10000
cmsis-dap-cli --target STM32F030C8 flash program --address 0x08000000 --file fw.hex --verify
cmsis-dap-cli --target STM32F030C8 read --address 0x08000000 --width u8 --count 0x100 --output dump.bin --format bin
```

### Script file

`flash.jlink`:

```text
connect
halt
reg pc
savebin C:/dump.bin 0x20000000 0x100
resume
q
```

```bash
cmsis-dap-cli --target STM32F030C8 script --file flash.jlink
```

### Machine-readable output

```bash
cmsis-dap-cli --json connect
cmsis-dap-cli --json read --address 0x20000000 --width u32 --count 2
```

```json
{"target":{"core_type":"Armv6m","core_count":1,"ap_count":1, ...}}
{"address":536870912,"width":"u32","values":[64000000,1]}
```

## Output and exit codes

- Default output is human-readable; `--json` prints the same structured
  payloads the MCP tools return. Logs always go to stderr.
- Exit codes: `0` success, `1` runtime error (probe/connect/flash failures),
  `2` usage error (unknown option, invalid value, missing argument).
- Monitor commands (`watch`, `rtt monitor`, `evr monitor`) print one line per
  sample/event (NDJSON in `--json` mode) and exit `0` after a clean Ctrl-C
  stop; `--count N` bounds the run for scripts and CI.

## REPL

`repl` starts an interactive shell that keeps one session open, so
halt/read/resume sequences work across lines:

```text
$ cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 repl
cmsis-dap-cli> connect
target: {"ap_count":1,"core_count":1,"core_type":"Armv6m", ...}
cmsis-dap-cli> halt
halted: true
cmsis-dap-cli> reg pc
pc = 0x800122A
cmsis-dap-cli> resume
running: true
cmsis-dap-cli> q
```

`?`/`help` shows the supported commands; `q`/`exit` quits. The REPL inherits
the global connection options, so `connect` uses them (no need to retype
`--target`). Flash erase/program run directly in the REPL too.

The REPL also exposes the live debugging commands with persistent watch state:

```text
watch add <name|0xADDR> [--width u8|u16|u32|u64] [--label TEXT]
watch list | watch remove <idx|name> | watch clear
watch interval <ms>
watch run [--count N] [--log-dir DIR | --log-file FILE]
rtt [info] [--channel 0,1] [--count N] [--interval-ms N] [--log-dir DIR | --log-file FILE]
evr [info] [--ctx 0..7] [--count N] [--log-dir DIR | --log-file FILE]
```

Monitors run until Ctrl-C (or `--count N`) and return to the prompt.

## Script commands

The script engine (used by `script` and the REPL) supports:

```text
connect | disconnect | init        session management
si swd|jtag                        interface
speed <khz>                        clock speed
device <name>                      target chip
adapter serial <id>                probe selection
halt | go | step                   execution
reset [run|halt]                   reset
reg <name> [<value>] | regs        core registers
mem8/16/32 <addr> [<n>] | mdb/mdh/mdw   read memory
w8/16/32 <addr> <value> | mwb/mwh/mww   write memory
savebin <file> <addr> <size>       export memory to a binary file
dump_image <file> <addr> <size>    alias of savebin
loadbin <file> <addr>              program a binary file
loadfile <file> [<addr>]           program axf/elf/bin/hex
flash write_image <file> [<addr>]  alias of loadfile
flash erase_sector <addr> <size>   erase a flash range
erase                              erase all flash
verifybin <file> [<addr>]          verify a binary file against memory
verify_image <file> [<addr>]       alias of verifybin
sleep <ms> | echo <text>           helpers
targets                            show connected target
? | help | q | exit                help and quit
```

## Tips and troubleshooting

- **Picking a chip**: built-in chips (`chip search NAME`) work with just
  `--target NAME`. For other chips, generate a target YAML once with
  `chip generate` and load it with `--target-yaml` (single-variant YAMLs
  auto-select; multi-variant YAMLs require `--target`).
- **Flash needs a chip definition**: without one, erase/program fail with a
  clear error instead of silently doing nothing.
- **Register reads need a halted core**: in one-shot mode, use `script`/`repl`
  so `halt` and `reg` share a session.
- **Flash cannot be written with `write`**: raw memory writes to flash are
  rejected; use `flash program`.
- **Numbers**: decimal or hex (`0x...`) everywhere.
