# Quickstart

## Native binary

Download the binary for your platform from the GitHub Releases page, then add
it to your MCP client (see [AI client configuration](./ai-clients.md)).

## npm

```bash
codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp
```

The npm package downloads the correct platform binary automatically.

## First session

1. `list_probes` to find your probe id.
2. `connect` with `{"protocol": "swd", "speed_khz": 1000}`.
3. `read_memory` / `write_memory` for raw access.
4. `halt`, then `read_core_register` (e.g. `pc`, `sp`, `lr`, `r0`).
5. `resume` when done.
6. `load_svd` with your own SVD path for named peripheral access.
7. `program_flash` only after starting the server with `--allow-destructive`.

Example (verified output on a CMSIS-DAP probe + Cortex-M0+ board):

```text
list_probes -> {"probes": [{"id": "0123456789AB", "product": "XV-Link CMSIS-DAP", ...}]}
connect {protocol: swd, speed_khz: 1000}
  -> {"target": {"core_type": "Armv6m", "core_count": 1, "ap_count": 1, "cpu_id": ..., "dp_id": ...}}
read_memory {address: 0x20000000, width: u32, count: 4}
  -> {"values": [64000000, 1, 3, 0]}
halt -> {"halted": true}
read_core_register {name: pc} -> {"value": 134228884}
resume -> {"running": true}
```

Logs go to stderr only; the MCP protocol runs over stdout.
