# CMSIS-DAP MCP

An MCP server that lets AI assistants operate CMSIS-DAP debug probes to access Cortex-M chip resources over SWD/JTAG.

- Generic Cortex-M support: no chip-specific adaptation needed for basic debug.
- Named peripheral access: load any CMSIS-SVD file at runtime (chip files are never bundled).
- Flash programming: requires a target description with a CMSIS-Pack flash algorithm.
- Zero runtime dependencies for end users: a single native binary, or install via npm.

## Features

| Area | Tools |
| --- | --- |
| Probe | `list_probes`, `get_probe_info`, `connect`, `disconnect`, `get_target_info` |
| Memory | `read_memory`, `write_memory` |
| Core | `read_core_register`, `write_core_register`, `halt`, `resume`, `step`, `set_breakpoint`, `clear_breakpoints`, `list_breakpoints`, `reset` |
| DAP | `read_dap`, `write_dap` |
| SVD | `load_svd`, `list_peripherals`, `read_peripheral`, `write_peripheral` |
| Flash | `erase_flash`, `program_flash` |

## Security

- Read-only tools are always available.
- Write and debug-control tools are marked as writes; your MCP client governs approval.
- `erase_flash` and `program_flash` are destructive and disabled unless the server is started with `--allow-destructive`.

## Quickstart

### Native binary

Download the binary for your platform from the GitHub Releases page, then configure:

```toml
[mcp_servers.cmsis-dap]
command = "/path/to/cmsis-dap-mcp"
args = ["--allow-destructive"] # optional
```

### npm

```bash
codex mcp add cmsis-dap-mcp -- npx -y cmsis-dap-mcp
```

## Using SVD files

```text
load_svd { "path": "/path/to/your-chip.svd" }
list_peripherals {}
read_peripheral { "peripheral": "GPIOA", "register": "ODR" }
write_peripheral { "peripheral": "GPIOA", "register": "ODR", "field": "ODR0", "value": 1 }
```

SVD files are provided by the user at runtime; this repository does not bundle chip-specific data.

## Flash programming

Flash tools require a target with a flash algorithm. Connect with a target name that probe-rs can resolve (built-in, or loaded from a CMSIS-Pack via a target description), then:

```text
program_flash { "address": 0x08000000, "data": [0x00, 0x11, ...] }
```

## Linux udev

On Linux, grant the current user access to debug probes once:

```text
# example for a CMSIS-DAP v1/v2 probe; adjust VID/PID to your hardware
SUBSYSTEM=="usb", ATTRS{idVendor}=="xxxx", ATTRS{idProduct}=="yyyy", MODE="0666"
```

## Development

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

## License

MIT OR Apache-2.0