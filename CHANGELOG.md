# Changelog

All notable changes are documented per release. Version numbers match the
`v*` tags; npm packages and platform binaries follow the same version.

## [v0.8.0] - 2026-08-30

### Features

- **Web Debug (browser UI)**: `cmsis-dap-cli web` starts a local Web Debug
  server (REST + WebSocket) serving a Keil-style Debug Workspace in the
  browser, built on the same engine as the CLI / MCP:
  - Execution control: connect/disconnect, run / halt / step / step-over /
    step-out / run-to-address, reset and reset-and-halt, snapshot.
  - CPU registers (HEX/DEC/BIN, PC highlight, bitfield view); memory viewer
    with u8/u16/u32 grouped display and in-place edit; hardware / flash
    software breakpoints and watchpoints.
  - ELF/AXF symbol explorer (search, drag-to-watch, ＋Watch), live watch,
    SVD peripheral explorer (read/write/decode, 全部刷新, in-place periodic
    monitor with configurable rate), flash erase/program/verify (BIN/HEX)
    with progress, RTT / EVR viewers, disassembly, call stack / locals
    (DWARF), source viewer, expression evaluator, PC-sampling profiler, SWO.
  - Dockable panel layout with presets (Quick / Full), a Window menu to
    show/hide every panel, and named layout configurations saved in the
    browser; layout persists across reloads.
  - Security: binds `127.0.0.1` by default; destructive tools (flash,
    flash-software breakpoints) require `--allow-destructive`. Flash
    programming needs a chip target (`--target-yaml` / `--target`) so flash
    regions and algorithms are defined.

### Fixed

- Web API `DELETE /api/breakpoints` now clears breakpoints (was 405 / 500).
- Status polling stops after session loss (no 409 storm; the UI returns to
  disconnected and stops hammering the server).
- Symbol list renders reliably after ELF/AXF upload; single-cell memory
  writes take effect; memory u8/u16/u32 width now changes the display
  grouping instead of just the read length.
- SVD peripheral monitoring no longer creates separate monitor blocks —
  "全部刷新" reads every readable register in place, and per-register
  "监控" refreshes the value in the same row.

### Documentation

- New `docs/src/web.md` (+ Chinese mirror) covering the Web Debug server,
  workspaces, panels and examples; `docs/implementation/ui-test-report.md`
  records the UI functional test matrix and bug fixes.
- Committed UI functional test suite under `tests/ui/` (Playwright + Edge).
- Pinned-version examples updated to `@0.8.0`; README Release badge synced
  to `v0.8.0`.

## [v0.7.0] - 2026-08-22

### Features

- Flash software breakpoints: `set_flash_breakpoint` /
  `clear_flash_breakpoints` / `list_flash_breakpoints` MCP tools and
  `bp set-flash` / `bp clear-flash` / `bp list-flash` CLI commands patch a
  Thumb `BKPT` (0xBE00) into flash and restore the original instruction on
  clear. Destructive (modifies flash) and gated behind
  `--allow-destructive`.
- Multi-core preparation: `connect` accepts an optional `core` index and
  the CLI a global `--core-index N`; `TargetInfo` now reports per-core
  descriptors (`cores`). All operations default to core 0.

## [v0.6.0] - 2026-08-22

### Features

- SWO/SWV trace: `start_swo` / `stop_swo` / `read_swo` MCP tools and
  `swo start` / `swo stop` / `swo monitor` CLI commands stream raw SWO
  bytes over the debug probe (no UART), with timestamped hex output and
  NDJSON (`host_ts`) in `--json` mode; `swo monitor` supports `--count`,
  `--interval-ms`, `--log-dir` / `--log-file`.
- Option bytes: `read_option_bytes` / `write_option_bytes` MCP tools and
  `option read` / `option write` CLI commands for chip option bytes
  (STM32: RDP, USER, DATA0, DATA1 from FLASH_OPTCR via raw DAP access).
  `write_option_bytes` is destructive (can lock the device) and requires
  `--allow-destructive`.
- Script memory writes (`w8` / `w16` / `w32`) that target a flash (NVM)
  region now route through the flash algorithm instead of a raw memory
  write, which is ignored by the flash controller. Flash writes are gated
  behind the destructive policy, preserve unwritten bytes in the erased
  sector, and report `flash_programmed` in the response.

- `swo monitor --json` emits `host_ts`, aligning its NDJSON output with
  the `watch` / `rtt` / `evr` monitors.

## [v0.5.0] - 2026-08-17

### Features

- Non-invasive debugging: `dump_cpu_state` (MCP tool) and `cmsis-dap-cli dump`
  take a CPU snapshot (registers, Cortex-M fault status registers, MSP/PSP
  stacks, optional memory samples) **without ever resetting** the target; core
  registers are read during a short halt and the previous run state is
  restored afterwards by default.
- Remote TCP server: `cmsis-dap-mcp --tcp PORT` and
  `cmsis-dap-cli tcp-server` serve a line-delimited JSON-RPC protocol
  (`read_memory`, `write_memory`, `read_core_register`, `halt`, `resume`,
  `step`, `reset`, `status`, `dump_cpu_state`, ...) over a shared session —
  no reconnect needed for follow-up requests.
- GDB Server: `cmsis-dap-mcp --gdb-port PORT` and `cmsis-dap-cli gdb-server`
  expose a GDB Remote Serial Protocol stub (ported from
  [probe-rs-tools](https://github.com/probe-rs/probe-rs), MIT OR Apache-2.0),
  including registers, memory, run/step, hardware breakpoints and target
  description. Attach is non-invasive (no reset).
- npm platform packages now cover win32/linux/darwin × x64/arm64 plus
  win32/linux × ia32 (32-bit), for both `cmsis-dap-mcp` and `cmsis-dap-cli`.
- REPl: `dump` command added.
- Runtime configuration: the server can be started with zero arguments
  (to-be-configured state) and fully configured at runtime — no restart
  needed. New MCP tools `get_config`, `update_config` (partial, validated,
  atomic updates of `allow_destructive` / `tcp_port` / `gdb_port`) and
  `reload_config` (re-apply `--config-file`). Optional `--config-file`
  JSON startup config with hot-reload file watcher (`config-watch`
  feature); CLI flags keep working and win over the file. TCP/GDB server
  tasks reconcile idempotently on every config change.
- `define_chip` MCP tool: register a custom/unknown chip at runtime from a
  Keil FLM flash algorithm file (FLM parsing, target YAML generation and
  registry injection all inside the MCP server — no standalone probe-rs
  CLI or external YAML files). After `define_chip`, `connect` attaches by
  chip name; SVD peripherals load separately via `load_svd`.

### Documentation

- New "Non-invasive debugging / remote TCP / GDB Server" sections with usage
  and examples, including source links (probe-rs gdb server, gdbstub, MCP
  spec, ARM CoreSight/SCB references, CMSIS-View docs).
- mdBook language switch (EN ↔ 中文) on every page; English site-url fix.
- npm READMEs: zero-config install guides for AI clients and standard
  `mcpServers` configuration.
- Documentation audit and restructure: split SUMMARY into a "User guide"
  group (introduction, quickstart, AI client config, tools, CLI, scripting,
  SWD/JTAG, SVD/Flash, security, troubleshooting) and a "Developer guide"
  group (architecture, development); Chinese mirror updated.
- `docs/src/tools.md`: added `dump_cpu_state` (non-invasive CPU snapshot)
  and made `run_script` explicit in the tool table; Chinese mirror updated.
- `docs/src/architecture.md`: added `gdb` and `remote` modules to the
  module responsibility table; Chinese mirror updated.
- `docs/src/development.md`: added "Code style", "Contributing", "Testing
  strategy" and "Documentation maintenance" sections; Chinese mirror updated.
- `npm/README.md`: removed a duplicate "## Quick start" heading, relocated
  the "Remote TCP, GDB and non-invasive debugging" section, and added
  Files/Scripts rows to the feature table to match the top-level README.
- `docs/src/quickstart.md` and `docs/src/ai-clients.md`: pinned-version
  examples updated from `@0.4.0` to `@0.5.0`; Chinese mirrors updated.
- npm `package.json` files (meta packages and 16 platform sub-packages)
  synced from `0.4.1` to `0.5.0` to match the Cargo workspace version.
- Top-level `README.md` Release badge `?branch=` synced from `v0.4.1` to
  `v0.5.0`.
- All new and modified documentation sections are mirrored between
  `docs/src/` and `docs/zh/src/`.
- `docs/src/tools.md` (+ zh mirror): documented `define_chip` (runtime chip
  registration from FLM) and the runtime configuration tools
  (`get_config` / `update_config` / `reload_config`); destructive level now
  notes the runtime enable path.
- `docs/src/security.md` (+ zh mirror): destructive tools can be enabled at
  runtime via `update_config`, not only via `--allow-destructive`.
- `docs/src/quickstart.md` (+ zh mirror): zero-argument startup documented;
  first-session steps updated for runtime destructive enable and
  `define_chip`.
- `docs/src/ai-clients.md` (+ zh mirror): new "Server command-line
  options" table covering all `cmsis-dap-mcp` flags, startup-only options
  and precedence rules.

## [v0.4.1] - 2026-08-16

### Features

- Live debugging: `watch` (variable polling), `rtt monitor` (SEGGER RTT) and
  `evr monitor` (CMSIS-View Event Recorder) with timestamped log export
  (CLI-only), ELF symbol lookup (`symbols`), and `--elf` support.

### Fixes

- Event Recorder decoding uses the official 16-byte record layout (event
  context instead of a non-stored level); unix build satisfies the
  `function-casts-as-integer` lint.

## [v0.2.0] - 2026-08-15

- Workspace split into `cmsis-dap-core` / `cmsis-dap-mcp` / `cmsis-dap-cli`;
  first CLI release.
