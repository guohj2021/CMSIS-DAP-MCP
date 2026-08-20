# CMSIS-DAP MCP

An open-source debug tool suite for **CMSIS-DAP** probes and **Cortex-M**
chips, providing an MCP server (for AI assistants) and a standalone CLI, both
built on the same engine over **SWD** or **JTAG**.

**New here?** Start with the [Getting Started](./getting-started.md) guide
for a step-by-step walkthrough from environment setup to your first connection.

## Two tools

- **cmsis-dap-mcp** --- an MCP (Model Context Protocol) server that lets AI
  assistants (Codex, Claude Code, opencode, etc.) drive your probe and target
  chip directly.
- **cmsis-dap-cli** --- a standalone command-line tool for humans, scripts and
  automation, no AI client needed.

## Core features

### Probe and session

| Tool | What it does |
| --- | --- |
| `list_probes` | Enumerate all connected CMSIS-DAP probes |
| `get_probe_info` | View probe details (product, serial, protocols, speeds) |
| `connect` | Connect to a target via SWD or JTAG; supports under-reset connect |
| `disconnect` | End the current session |
| `get_target_info` | View target info (core type, CPUID, memory regions) |

### Memory access

| Tool | What it does |
| --- | --- |
| `read_memory` | Read memory (u8/u16/u32/u64); export a range as bin/hex file |
| `write_memory` | Write memory |
| `verify_memory` | Read back and compare against expected data; report mismatches |

### Core control

| Tool | What it does |
| --- | --- |
| `read_core_register` / `write_core_register` | Read/write core registers (pc, sp, lr, r0-r15, ...) |
| `list_core_registers` | List all registers available on the target |
| `get_core_status` | Query core state (running/halted/sleeping/locked up) |
| `halt` / `resume` / `step` | Pause / resume / single-step execution |
| `reset` | Reset the target; continue or halt after reset |

### Breakpoints and watchpoints

| Tool | What it does |
| --- | --- |
| `set_breakpoint` / `clear_breakpoints` / `list_breakpoints` | Hardware breakpoint management |
| `set_watchpoint` / `clear_watchpoints` / `list_watchpoints` | DWT data watchpoints (read/write/rw trigger) |

### DAP raw access

| Tool | What it does |
| --- | --- |
| `read_dap` / `write_dap` | Direct DP/AP register read/write (advanced debugging) |

### SVD named peripherals

| Tool | What it does |
| --- | --- |
| `load_svd` | Load any CMSIS-SVD file at runtime |
| `list_peripherals` | List all loaded peripherals |
| `read_peripheral` / `write_peripheral` | Read/write peripheral registers and bitfields by name (read-modify-write) |

### Flash programming

| Tool | What it does |
| --- | --- |
| `erase_flash` | Erase flash by sector (only sectors overlapping the requested range) |
| `program_flash` | Program firmware from elf/axf/bin/hex files; optional read-back verify |

### Chip definition

| Tool | What it does |
| --- | --- |
| `define_chip` (MCP) | Register an unknown chip at runtime from a Keil FLM file |
| `chip generate` (CLI) | Generate a probe-rs target YAML from an FLM file |
| `chip list` / `chip search` | List or search the built-in chip library |

### Script engine

| Tool | What it does |
| --- | --- |
| `run_script` (MCP) / `script` (CLI) | Execute J-Link Commander / OpenOCD style debug scripts |

### Non-invasive debugging

| Tool | What it does |
| --- | --- |
| `dump_cpu_state` (MCP) / `dump` (CLI) | Take a CPU snapshot without resetting: registers, fault status, stacks, memory |

### Remote access

| Feature | Description |
| --- | --- |
| TCP JSON-RPC server | `--tcp PORT` (MCP) or `tcp-server` (CLI): line-delimited remote protocol |
| GDB server | `--gdb-port PORT` (MCP) or `gdb-server` (CLI): GDB Remote Serial Protocol stub |

### Runtime configuration

| Tool | What it does |
| --- | --- |
| `get_config` | View current runtime configuration |
| `update_config` | Update config at runtime (destructive gate, TCP/GDB ports) without restart |
| `reload_config` | Re-apply the config file given at startup |

### Security

Three-tier security: read-only tools are always available; write tools are
governed by the MCP client approval policy; destructive tools (flash
erase/program) are disabled by default and must be explicitly enabled.

## CLI live debugging (CLI-only)

| Feature | Description |
| --- | --- |
| `watch` | Poll variables by address or ELF symbol with configurable refresh; timestamped log export |
| `rtt monitor` | Read SEGGER RTT up-channel logs over SWD/JTAG --- no UART needed |
| `evr monitor` | Decode CMSIS-View Event Recorder events --- no trace hardware needed |
| `repl` | Interactive shell that keeps one session open |

## Highlights

- **Generic Cortex-M support**: standard cores work without chip-specific adaptation
- **Runtime chip definition**: register unknown chips from FLM files; no pre-built YAML needed
- **Zero-argument startup**: server starts with no flags and is fully configurable at runtime
- **Zero dependencies for end users**: `npx -y cmsis-dap-mcp` or a single native binary
- **Cross-platform**: Windows / Linux / macOS

## Documentation

- [Getting Started](./getting-started.md) --- complete setup and first-connection tutorial
- [Quickstart](./quickstart.md) --- MCP server quick setup
- [AI client configuration](./ai-clients.md) --- Codex / Claude Code / opencode setup
- [Tools](./tools.md) --- full MCP tool reference
- [CLI](./cli.md) --- full CLI command reference
- [Scripting](./scripting.md) --- J-Link / OpenOCD style scripts
- [SWD and JTAG](./swd-jtag.md) --- protocol selection
- [SVD and Flash](./svd-flash.md) --- peripheral access and flash workflows
- [Security](./security.md) --- security model and configuration
- [Troubleshooting](./troubleshooting.md) --- common issues

Chinese documentation: <https://guohj2021.github.io/CMSIS-DAP-MCP/zh/>
