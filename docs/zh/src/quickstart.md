# 快速开始

## 原生二进制

从 GitHub Releases 下载对应平台的二进制，然后配置到你的 MCP 客户端
（见 [AI 客户端配置](./ai-clients.md)）。

## npm

```bash
codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp
```

npm 包会自动下载对应平台的二进制。

## 第一次会话

1. `list_probes` 查找探针 id。
2. `connect`，参数 `{"protocol": "swd", "speed_khz": 1000}`。
3. `read_memory` / `write_memory` 原始内存访问。
4. `halt`，然后 `read_core_register`（例如 `pc`、`sp`、`lr`、`r0`）。
5. 完成后 `resume`。
6. `load_svd` 加载你自己的 SVD 文件，进行命名外设访问。
7. 只有以 `--allow-destructive` 启动服务器后才可 `program_flash`。

示例（CMSIS-DAP 探针 + Cortex-M0+ 开发板实测输出）：

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

日志只写入 stderr；MCP 协议运行在 stdout 上。
