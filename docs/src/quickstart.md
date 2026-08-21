# Quickstart

## npm (recommended)

Install nothing — let your MCP client launch the server with `npx`:

```bash
codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp
```

The npm package `cmsis-dap-mcp` downloads the correct platform binary
automatically on first launch and caches it afterwards.

To pin a version:

```bash
codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp@0.6.0
```

## Native binary

Download the binary for your platform from the GitHub Releases page, then
point the client at it:

```bash
codex mcp add cmsis-dap -- /path/to/cmsis-dap-mcp --log-level warn
```

This is the standard way to run an unpublished or locally built server, or
when you need an exact, offline-pinned binary.

## Configuration styles

MCP clients can start a stdio server in three equivalent ways. The
`npx` form is the standard for published packages; the local-binary form is
equivalent and used for local builds.

| Style | Example | Best for |
| --- | --- | --- |
| `npx` package | `command = "npx", args = ["-y", "cmsis-dap-mcp"]` | Published releases; updates with npm |
| Local binary | `command = "/path/to/cmsis-dap-mcp"` | Local builds, offline, exact version |
| Remote URL | `url = "https://..."` | Streamable-HTTP servers (not supported by this project) |

All three clients covered on the [AI client configuration](./ai-clients.md)
page accept both the `npx` form and a local binary path; the server behaves
identically either way.

## First session

The server can be started with zero arguments — it enters a
to-be-configured state where all read/write tools work and destructive
tools stay gated until enabled (see step 7).

1. `list_probes` to find your probe id.
2. `connect` with `{"protocol": "swd", "speed_khz": 1000}`.
3. `read_memory` / `write_memory` for raw access.
4. `halt`, then `read_core_register` (e.g. `pc`, `sp`, `lr`, `r0`).
5. `resume` when done.
6. `load_svd` with your own SVD path for named peripheral access.
7. `program_flash` / `erase_flash` require destructive mode: start the
   server with `--allow-destructive`, **or** call
   `update_config {"allow_destructive": true}` at runtime (no restart
   needed).
8. For a chip not built into probe-rs, call `define_chip` with a Keil FLM
   file before `connect` (see [Tools](./tools.md)).

Example (verified output on a CMSIS-DAP probe + Cortex-M0+ board):

```text
list_probes -> {"probes": [{"id": "0123456789AB", "product": "CMSIS-DAP", ...}]}
connect {protocol: swd, speed_khz: 1000}
  -> {"target": {"core_type": "Armv6m", "core_count": 1, "ap_count": 1, "cpu_id": ..., "dp_id": ...}}
read_memory {address: 0x20000000, width: u32, count: 4}
  -> {"values": [64000000, 1, 3, 0]}
halt -> {"halted": true}
read_core_register {name: pc} -> {"value": 134228884}
resume -> {"running": true}
```

## CLI quick start

The standalone `cmsis-dap-cli` shares the same engine and auto-connects with
the global options (`--probe-id`, `--target`, `--target-yaml`, ...):

```bash
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 connect
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 read --address 0x20000000 --width u32 --count 4
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 --elf fw.axf watch counter --interval-ms 200 --count 0
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 --elf fw.axf rtt monitor --count 0
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 --elf fw.axf evr monitor --count 0
```

Use `repl` to keep one session open (halt/read/resume across lines, or run the
watch/RTT/Event Recorder monitors after `reset run`). See the
[CLI reference](./cli.md) for the full command set.

Logs go to stderr only; the MCP protocol runs over stdout.
