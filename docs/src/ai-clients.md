# AI client configuration

The server speaks MCP over stdio. The configurations below were verified
locally with Codex, Claude Code and opencode. Replace
`/path/to/cmsis-dap-mcp` with your binary path, or use
`npx -y cmsis-dap-mcp`.

## Codex

```bash
codex mcp add cmsis-dap -- /path/to/cmsis-dap-mcp --log-level warn
```

Or add to `~/.codex/config.toml`:

```toml
[mcp_servers.cmsis-dap]
command = "/path/to/cmsis-dap-mcp"
args = ["--log-level", "warn"] # optional
```

Verify with `codex mcp list`. The Codex desktop app loads the server when a
new session starts.

## Claude Code

```bash
claude mcp add --scope local cmsis-dap -- /path/to/cmsis-dap-mcp
```

Verify with `claude mcp list` (shows `√ Connected`).

## opencode

```bash
opencode mcp add cmsis-dap -- /path/to/cmsis-dap-mcp --log-level warn
```

Or add to `~/.config/opencode/opencode.jsonc`:

```jsonc
"cmsis-dap": {
  "type": "local",
  "command": ["/path/to/cmsis-dap-mcp", "--log-level", "warn"],
  "enabled": true
}
```

Verify with `opencode mcp list`.

## Other MCP clients

```json
{
  "mcpServers": {
    "cmsis-dap": {
      "command": "/path/to/cmsis-dap-mcp",
      "args": ["--log-level", "warn"]
    }
  }
}
```

## End-to-end example (verified)

The same task below was executed successfully by Claude Code and opencode
against a real CMSIS-DAP probe:

```text
1. list_probes
2. connect {protocol: swd, speed_khz: 1000}
3. read_memory {address: 0x20000000, width: u32, count: 4}
4. halt
5. read_core_register {name: pc}
6. resume
```

Observed results:

```text
probe id : 0123456789AB (XV-Link CMSIS-DAP, vendor 0x0416)
memory   : [64000000, 1, 3, 0]
pc       : 134228884 (0x08002B94)
```

Notes:

- When passing arguments from a model, use decimal integers or strings; some
  clients reject hex literals in JSON arguments (e.g. `0x20000000`). Decimal
  `536870912` is equivalent.
- Write tools such as `connect`, `halt` and `resume` may be governed by the
  client approval policy.
