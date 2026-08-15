# Quickstart

## Native binary

Download the binary for your platform from the GitHub Releases page, then add it to your MCP client:

```toml
[mcp_servers.cmsis-dap]
command = "/path/to/cmsis-dap-mcp"
args = ["--allow-destructive"] # optional
```

## npm

```bash
codex mcp add cmsis-dap-mcp -- npx -y cmsis-dap-mcp
```

## First session

1. `list_probes` to find your probe id.
2. `connect` with the probe id (protocol `swd` by default).
3. `read_memory` / `write_memory` for raw access.
4. `load_svd` with your own SVD path for named peripheral access.
5. `program_flash` only after starting the server with `--allow-destructive`.

Logs go to stderr only; the MCP protocol runs over stdout.