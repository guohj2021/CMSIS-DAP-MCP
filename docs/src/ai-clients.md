# AI client configuration

The server speaks MCP over stdio. The standard way to configure it is the
`npx` form, which runs the published npm package. To run a locally built
binary instead, replace `npx -y cmsis-dap-mcp` with your binary path — the
server behaves identically.

## Configuration styles

There are three ways to point an MCP client at a server:

| Style | Example | When to use |
| --- | --- | --- |
| `npx` package (standard) | `command = "npx", args = ["-y", "cmsis-dap-mcp"]` | Published releases; first launch downloads and caches the package |
| Local binary | `command = "/path/to/cmsis-dap-mcp"` | Unpublished or locally built servers, offline use, exact version pinning |
| Remote URL | `url = "https://..."` | Streamable-HTTP MCP servers (not supported by this project yet) |

To pin a version with `npx`: `npx -y cmsis-dap-mcp@0.3.0`. If you are
developing this repository, point the client at `target/release/cmsis-dap-mcp`
so the freshly built binary is used without publishing.

## Codex

```bash
codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp
```

Or add to `~/.codex/config.toml`:

```toml
[mcp_servers.cmsis-dap]
command = "npx"
args = ["-y", "cmsis-dap-mcp"]
```

For a local build, use `command = "/path/to/cmsis-dap-mcp"`. Verify with
`codex mcp list`. The Codex desktop app loads the server when a new session
starts.

## Claude Code

```bash
claude mcp add --scope local cmsis-dap -- npx -y cmsis-dap-mcp
```

For a local build, replace `npx -y cmsis-dap-mcp` with the binary path.
Verify with `claude mcp list` (shows `√ Connected`).

## opencode

```bash
opencode mcp add cmsis-dap -- npx -y cmsis-dap-mcp
```

Or add to `~/.config/opencode/opencode.jsonc`:

```jsonc
"cmsis-dap": {
  "type": "local",
  "command": ["npx", "-y", "cmsis-dap-mcp"],
  "enabled": true
}
```

For a local build, replace the `command` array with
`["/path/to/cmsis-dap-mcp", "--log-level", "warn"]`. Verify with
`opencode mcp list`.

## Other MCP clients

```json
{
  "mcpServers": {
    "cmsis-dap": {
      "command": "npx",
      "args": ["-y", "cmsis-dap-mcp"]
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
- If the tools do not appear, restart the client after adding the server.
