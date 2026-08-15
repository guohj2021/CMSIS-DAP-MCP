# AI 客户端配置

服务器通过 stdio 使用 MCP 协议。以下配置已在本地用 Codex、Claude Code 和
opencode 实测。请把 `/path/to/cmsis-dap-mcp` 换成你的二进制路径，或使用
`npx -y cmsis-dap-mcp`。

## Codex

```bash
codex mcp add cmsis-dap -- /path/to/cmsis-dap-mcp --log-level warn
```

或写入 `~/.codex/config.toml`：

```toml
[mcp_servers.cmsis-dap]
command = "/path/to/cmsis-dap-mcp"
args = ["--log-level", "warn"] # 可选
```

用 `codex mcp list` 确认。Codex 桌面端会在新会话启动时加载该服务器。

## Claude Code

```bash
claude mcp add --scope local cmsis-dap -- /path/to/cmsis-dap-mcp
```

用 `claude mcp list` 确认（显示 `√ Connected`）。

## opencode

```bash
opencode mcp add cmsis-dap -- /path/to/cmsis-dap-mcp --log-level warn
```

或写入 `~/.config/opencode/opencode.jsonc`：

```jsonc
"cmsis-dap": {
  "type": "local",
  "command": ["/path/to/cmsis-dap-mcp", "--log-level", "warn"],
  "enabled": true
}
```

用 `opencode mcp list` 确认。

## 其他 MCP 客户端

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

## 端到端示例（已实测）

以下任务已由 Claude Code 和 opencode 在真实 CMSIS-DAP 探针上成功执行：

```text
1. list_probes
2. connect {protocol: swd, speed_khz: 1000}
3. read_memory {address: 0x20000000, width: u32, count: 4}
4. halt
5. read_core_register {name: pc}
6. resume
```

实测结果：

```text
探针 id : 0123456789AB（XV-Link CMSIS-DAP，vendor 0x0416）
内存    : [64000000, 1, 3, 0]
pc      : 134228884（0x08002B94）
```

说明：

- 模型传参时请使用十进制整数或字符串；部分客户端会拒绝 JSON 参数里的十六进制
  字面量（如 `0x20000000`）。十进制 `536870912` 等价。
- `connect`、`halt`、`resume` 等写工具可能受客户端审批策略约束。
