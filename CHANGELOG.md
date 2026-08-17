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

### Documentation

- New "Non-invasive debugging / remote TCP / GDB Server" sections with usage
  and examples, including source links (probe-rs gdb server, gdbstub, MCP
  spec, ARM CoreSight/SCB references, CMSIS-View docs).
- mdBook language switch (EN ↔ 中文) on every page; English site-url fix.
- npm READMEs: zero-config install guides for AI clients and standard
  `mcpServers` configuration.

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
