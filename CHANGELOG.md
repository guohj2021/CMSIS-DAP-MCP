# Changelog

All notable changes are documented per release. Version numbers match the
`v*` tags; npm packages and platform binaries follow the same version.

## [v0.5.0] - unreleased

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
