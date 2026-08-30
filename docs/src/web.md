# Web Debug (browser UI)

`cmsis-dap-cli web` starts a local Web Debug server that exposes the same
debug engine as a browser-based Debug Workspace — connect to a CMSIS-DAP
probe, inspect registers / memory / peripherals, set breakpoints, watch
variables live, read RTT / EVR output, disassemble, inspect the call stack
and locals (DWARF), and program flash — all without Keil / IAR / Eclipse /
VS Code.

## Start

```bash
cmsis-dap-cli web
```

By default the server binds `127.0.0.1:8080` and opens a single-page
application. Options:

| Flag | Meaning |
| --- | --- |
| `--host` | Bind host (default `127.0.0.1`; use `0.0.0.0` only when you intend to expose the debugger) |
| `--port` | Bind port (default `8080`) |
| `--allow-destructive` | Enable flash erase/program and flash software breakpoints |
| `--flash-timeout` | Flash operation timeout in seconds (default 600) |

All other global options (`--target`, `--target-yaml`, `--flm`, `--probe-id`,
`--svd`, `--elf`, `--protocol`, `--speed-khz`, `--under-reset`,
`--core-index`) are honoured as defaults for the web session.

Example for a custom chip defined by a Keil FLM:

```bash
cmsis-dap-cli web --target-yaml target/demo.yaml --probe-id <serial> --allow-destructive
```

## Workspaces

- **Debug** — Dockable panels: Target/Symbols, Code/Disassembly, CPU
  Registers, Watch/Live Watch, Memory, Peripheral (SVD), Breakpoints,
  Call Stack, Locals, RTT, EVR, SWO and Console. Quick Debug and Full Debug
  are layout presets of the same workspace; the layout is persisted in the
  browser.
- **Flash** — drop a BIN/HEX file, analyse it (address ranges), then
  Erase / Program / Verify / Reset / Run with live progress. Flashing is
  destructive and always requires an explicit confirmation.

## Highlights

- **Symbols → Watch / Breakpoint** — upload an ELF/AXF (debug info only,
  never flashed), search functions/variables, drag a symbol into Watch, or
  set a breakpoint by symbol name.
- **Live Watch** — variable values stream while the target runs; pick a
  refresh rate per item (50 ms … 1 s). Register-backed items are only read
  while halted.
- **Peripheral explorer** — upload an SVD file, browse registers with
  bit-field decode, and periodically refresh only the registers you monitor.
- **Call Stack + Locals** — real DWARF CFI unwinding and DWARF variable
  evaluation (types: base/pointer/typedef/struct/array/enum). Frames that
  have no unwind info, or values that cannot be read, are reported honestly
  ("unavailable") — never fabricated.
- **Expression** — evaluate C-like expressions over symbols, registers
  (`$pc`, `$sp`) and memory (`*(0x20000000)`).
- **Fault** — when the core halts, CFSR/HFSR/DFSR/MMFAR/BFAR are shown and a
  Fault banner appears if any are set.
- **Command Palette** — `Ctrl+Shift+P` to run common commands.

## WebSocket events

The UI receives incremental events over `/ws`: `target_state_changed`,
`live_watch_value_changed`, `peripheral_value_changed`, `rtt_output`,
`evr_event`, `flash_progress`, `flash_complete`, `probe_lost`, and more.
`probe_lost` invalidates the session immediately (a USB removal never leaves
the UI "connected").

## Security

The server binds to `127.0.0.1` by default and exposes the debugger only
locally. Destructive operations (flash) are gated behind
`--allow-destructive` and require an explicit confirmation in the UI.
