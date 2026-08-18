# CMSIS-DAP MCP

Two tools for working with CMSIS-DAP debug probes and Cortex-M chips over
**SWD** or **JTAG**, built on the same engine:

- **cmsis-dap-mcp** — an MCP (Model Context Protocol) server that lets AI
  assistants operate the probe;
- **cmsis-dap-cli** — a standalone command-line tool for humans, scripts and
  automation.

Both enumerate probes, connect over SWD/JTAG, read/write memory and core
registers, control execution, access named peripherals via SVD files, program
flash from firmware files, and run J-Link / OpenOCD style debug scripts.
The CLI additionally ships live debugging: `watch` (poll variables with a
configurable refresh interval), `rtt monitor` (SEGGER RTT logs) and
`evr monitor` (CMSIS-View Event Recorder) all run over SWD/JTAG — no UART —
with timestamped log export.

The CLI also exposes non-invasive debugging and remote access endpoints
introduced in v0.5.0: `dump` / MCP `dump_cpu_state` (CPU snapshot without
resetting), `--tcp` / `tcp-server` (remote JSON-RPC over TCP) and
`--gdb-port` / `gdb-server` (GDB Remote Serial Protocol stub).

- Generic Cortex-M support: standard cores work without chip-specific
  adaptation.
- Named peripheral access: load any CMSIS-SVD file at runtime; chip files are
  never bundled.
- Flash programming: requires a target description with a CMSIS-Pack flash
  algorithm.
- Zero runtime dependencies for end users: one native binary, or install via
  npm.
- Cross-platform: Windows / Linux / macOS.

This site covers setup, AI client configuration, the full MCP tool and CLI
references, SWD/JTAG selection, SVD/Flash workflows and security guidance.

Chinese documentation: <https://guohj2021.github.io/CMSIS-DAP-MCP/zh/>
