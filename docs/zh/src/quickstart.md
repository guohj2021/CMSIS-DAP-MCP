# 快速开始

## npm（推荐）

无需安装：让 MCP 客户端用 `npx` 启动服务器即可：

```bash
codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp
```

`cmsis-dap-mcp` npm 包会在首次启动时自动下载对应平台的二进制并缓存。

固定版本：

```bash
codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp@0.5.0
```

## 原生二进制

从 GitHub Releases 下载对应平台二进制，然后让客户端指向它：

```bash
codex mcp add cmsis-dap -- /path/to/cmsis-dap-mcp --log-level warn
```

这是运行未发布或本地构建服务器的标准方式，也适合需要离线、精确固定版本的
场景。

## 配置方式

MCP 客户端有三种等价的 stdio 配置写法。`npx` 是已发布包的标准写法；本地
二进制写法与之等价，用于本地构建。

| 方式 | 示例 | 适用场景 |
| --- | --- | --- |
| `npx` 包 | `command = "npx", args = ["-y", "cmsis-dap-mcp"]` | 已发布版本；随 npm 更新 |
| 本地二进制 | `command = "/path/to/cmsis-dap-mcp"` | 本地构建、离线、精确版本 |
| 远程 URL | `url = "https://..."` | Streamable-HTTP 服务器（本项目暂不支持） |

[AI 客户端配置](./ai-clients.md) 页中三个客户端都同时接受 `npx` 与本地二进制
路径两种写法，服务器行为完全一致。

## 第一次会话

服务器可零参数启动——进入待配置态：所有读/写工具可用，破坏性工具保持
门控，直到按第 7 步启用。

1. `list_probes` 查找探针 id。
2. `connect`，参数 `{"protocol": "swd", "speed_khz": 1000}`。
3. `read_memory` / `write_memory` 原始内存访问。
4. `halt`，然后 `read_core_register`（例如 `pc`、`sp`、`lr`、`r0`）。
5. 完成后 `resume`。
6. `load_svd` 加载你自己的 SVD 文件，进行命名外设访问。
7. `program_flash` / `erase_flash` 需要破坏性模式：启动时加
   `--allow-destructive`，**或**运行时调用
   `update_config {"allow_destructive": true}`（无需重启）。
8. 对 probe-rs 未内置的芯片，先调用 `define_chip` 传入 Keil FLM 文件
   再 `connect`（见[工具参考](./tools.md)）。

示例（CMSIS-DAP 探针 + Cortex-M0+ 开发板实测输出）：

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

日志只写入 stderr；MCP 协议运行在 stdout 上。

## CLI 快速上手

独立命令行工具 `cmsis-dap-cli` 与服务器共用同一引擎，会自动使用全局连接
参数（`--probe-id`、`--target`、`--target-yaml` 等）：

```bash
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 connect
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 read --address 0x20000000 --width u32 --count 4
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 --elf fw.axf watch counter --interval-ms 200 --count 0
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 --elf fw.axf rtt monitor --count 0
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 --elf fw.axf evr monitor --count 0
```

用 `repl` 保持单一会话（halt/读/恢复跨行执行，或在 `reset run` 后运行
watch/RTT/Event Recorder 监控）。完整命令参考见[命令行工具](./cli.md)。
