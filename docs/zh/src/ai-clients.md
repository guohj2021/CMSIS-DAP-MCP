# AI 客户端配置

服务器通过 stdio 使用 MCP 协议。标准配置方式是 `npx` 形式（运行已发布的 npm
包）；要运行本地构建的二进制，把 `npx -y cmsis-dap-mcp` 换成二进制路径即可，
服务器行为完全一致。

## 配置方式

把 MCP 客户端指向服务器有三种写法：

| 方式 | 示例 | 适用场景 |
| --- | --- | --- |
| `npx` 包（标准） | `command = "npx", args = ["-y", "cmsis-dap-mcp"]` | 已发布版本；首次启动下载并缓存 |
| 本地二进制 | `command = "/path/to/cmsis-dap-mcp"` | 未发布或本地构建、离线、精确版本 |
| 远程 URL | `url = "https://..."` | Streamable-HTTP MCP 服务器（本项目暂不支持） |

用 `npx` 固定版本：`npx -y cmsis-dap-mcp@0.5.0`。开发本仓库时，把客户端
指向 `target/release/cmsis-dap-mcp`，即可使用刚构建的二进制而无需发布。

## Codex

```bash
codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp
```

或写入 `~/.codex/config.toml`：

```toml
[mcp_servers.cmsis-dap]
command = "npx"
args = ["-y", "cmsis-dap-mcp"]
```

本地构建时用 `command = "/path/to/cmsis-dap-mcp"`。用 `codex mcp list`
确认；Codex 桌面端在新会话启动时加载该服务器。

## Claude Code

```bash
claude mcp add --scope local cmsis-dap -- npx -y cmsis-dap-mcp
```

本地构建时把 `npx -y cmsis-dap-mcp` 换成二进制路径。用 `claude mcp list`
确认（显示 `√ Connected`）。

## opencode

```bash
opencode mcp add cmsis-dap -- npx -y cmsis-dap-mcp
```

或写入 `~/.config/opencode/opencode.jsonc`：

```jsonc
"cmsis-dap": {
  "type": "local",
  "command": ["npx", "-y", "cmsis-dap-mcp"],
  "enabled": true
}
```

本地构建时把 `command` 数组换成 `["/path/to/cmsis-dap-mcp", "--log-level",
"warn"]`。用 `opencode mcp list` 确认。

## 其他 MCP 客户端

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
- 添加服务器后如果工具不出现，请重启客户端。
