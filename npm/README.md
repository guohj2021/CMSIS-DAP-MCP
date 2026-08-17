# CMSIS-DAP MCP

[English](#english) · [中文](#chinese)

---

<a id="english"></a>
# English

An MCP (Model Context Protocol) server that lets AI assistants operate
CMSIS-DAP debug probes and access Cortex-M chip resources over **SWD** or
**JTAG**.

- Generic Cortex-M support: standard cores work without chip-specific
  adaptation.
- Named peripheral access: load any CMSIS-SVD file at runtime; chip files are
  never bundled.
- Flash programming: requires a target description with a CMSIS-Pack flash
  algorithm.
- Zero runtime dependencies for end users: one native binary, or install via
  npm.
- Cross-platform: Windows / Linux / macOS, distributed as a single binary via
  npm platform packages and GitHub Releases.

## Features

| Area | Tools |
| --- | --- |
| Probe | `list_probes`, `get_probe_info`, `connect`, `disconnect`, `get_target_info` |
| Memory | `read_memory`, `write_memory`, `verify_memory` |
| Core | `read_core_register`, `write_core_register`, `list_core_registers`, `get_core_status`, `halt`, `resume`, `step`, `reset` |
| Breakpoints | `set_breakpoint`, `clear_breakpoints`, `list_breakpoints` |
| Watchpoints | `set_watchpoint`, `clear_watchpoints`, `list_watchpoints` |
| DAP | `read_dap`, `write_dap` |
| SVD | `load_svd`, `list_peripherals`, `read_peripheral`, `write_peripheral` |
| Flash | `erase_flash`, `program_flash` |

`reset` supports `mode: "run"` (reset and continue) or `mode: "halt"` (reset
and halt). `connect` supports `under_reset` for locked or non-responsive
targets. `program_flash` supports `verify: true` for read-back checking, and
`erase_flash` erases only the requested address range (sector erase).

## Installation

### Native binary

Download the binary for your platform from the GitHub Releases page, then
configure your MCP client (see below).

### npm

```bash
codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp
```

The npm package `cmsis-dap-mcp` downloads the correct platform binary
automatically (win32/linux/darwin × x64/arm64).

## AI client configuration

The server speaks MCP over stdio. Below are verified configurations for
Codex, Claude Code and opencode. Replace `/path/to/cmsis-dap-mcp` with your
binary path or use `npx -y cmsis-dap-mcp`.

### Codex

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

### Claude Code

```bash
claude mcp add --scope local cmsis-dap -- /path/to/cmsis-dap-mcp
```

Verify with `claude mcp list` (shows `√ Connected`).

### opencode

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

### Other MCP clients

Any MCP-compatible client can use a stdio server:

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

## Quick start (verified on hardware)

The following workflow was verified end to end with a CMSIS-DAP probe and a
Cortex-M0+ board, driven through Claude Code, opencode and raw MCP stdio:

1. `list_probes` to find your probe id.
2. `connect` with `{"protocol": "swd", "speed_khz": 1000}`.
3. `read_memory` / `write_memory` for raw access.
4. `halt`, then `read_core_register` (e.g. `pc`, `sp`, `lr`, `r0`).
5. `resume` when done.
6. `load_svd` with your own SVD path for named peripheral access.
7. `program_flash` only after starting the server with `--allow-destructive`.

Example session (actual output):

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

## Using SVD files

```text
load_svd { "path": "/path/to/your-chip.svd" }
list_peripherals {}
read_peripheral { "peripheral": "GPIOA", "register": "ODR" }
write_peripheral { "peripheral": "GPIOA", "register": "ODR", "field": "ODR0", "value": 1 }
```

SVD files are provided by the user at runtime; this repository does not bundle
chip-specific data.

## Flash programming

Flash tools require a target with a flash algorithm. Provide a probe-rs target
description YAML at startup:

```bash
cmsis-dap-mcp --target-yaml /path/to/your-target.yaml --allow-destructive
```

Then connect with the target name from the YAML and program:

```text
connect { "protocol": "swd", "target": "YourChip" }
erase_flash { "address": 0x08000000, "size": 0x1000 }
program_flash { "address": 0x08000000, "data": [0x00, 0x11, ...], "verify": true }
```

`verify: true` reads the data back after programming. `erase_flash` erases
only the sectors overlapping the requested range.

## Security

- Read-only tools are always available.
- Write and debug-control tools are marked as writes; your MCP client governs
  approval.
- `erase_flash` and `program_flash` are destructive and disabled unless the
  server is started with `--allow-destructive`.

Flash erasing, option-byte changes, read-protection and debug unlock can
permanently damage a device or make it unrecoverable. Only enable destructive
mode when you explicitly intend to reprogram the target.

## Linux udev

On Linux, grant the current user access to debug probes once:

```text
# example for a CMSIS-DAP v1/v2 probe; adjust VID/PID to your hardware
SUBSYSTEM=="usb", ATTRS{idVendor}=="xxxx", ATTRS{idProduct}=="yyyy", MODE="0666"
```

## Development

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
mdbook build docs        # English docs
mdbook build docs/zh     # Chinese docs
```

## Documentation

Full documentation (English and Chinese) is published on GitHub Pages:

- English: <https://guohj2021.github.io/CMSIS-DAP-MCP/>
- 中文: <https://guohj2021.github.io/CMSIS-DAP-MCP/zh/>

## License

MIT OR Apache-2.0

---

<a id="chinese"></a>
# 中文

一个 MCP（模型上下文协议）服务器，让 AI 助手可以直接操作 CMSIS-DAP 调试探针，
通过 **SWD** 或 **JTAG** 访问 Cortex-M 芯片资源。

- 通用 Cortex-M 支持：标准内核无需芯片适配即可调试。
- 命名外设访问：运行时加载任意 CMSIS-SVD 文件；仓库不捆绑任何芯片文件。
- Flash 编程：需要带 CMSIS-Pack 烧写算法的目标描述。
- 终端用户零运行时依赖：单个原生二进制，或通过 npm 安装。
- 跨平台：Windows / Linux / macOS，通过 npm 平台包和 GitHub Releases 分发。

## 功能

| 分类 | 工具 |
| --- | --- |
| 探针 | `list_probes`、`get_probe_info`、`connect`、`disconnect`、`get_target_info` |
| 内存 | `read_memory`、`write_memory`、`verify_memory` |
| 内核 | `read_core_register`、`write_core_register`、`list_core_registers`、`get_core_status`、`halt`、`resume`、`step`、`reset` |
| 断点 | `set_breakpoint`、`clear_breakpoints`、`list_breakpoints` |
| 数据观察点 | `set_watchpoint`、`clear_watchpoints`、`list_watchpoints` |
| DAP | `read_dap`、`write_dap` |
| SVD | `load_svd`、`list_peripherals`、`read_peripheral`、`write_peripheral` |
| Flash | `erase_flash`、`program_flash` |

`reset` 支持 `mode: "run"`（复位后继续运行）或 `mode: "halt"`（复位后暂停）；
`connect` 支持 `under_reset`（用于锁定或无响应的目标）；`program_flash` 支持
`verify: true` 烧写后读回校验；`erase_flash` 只擦除请求的地址范围（扇区擦除）。

## 安装

### 原生二进制

从 GitHub Releases 下载对应平台的二进制，然后按下文配置你的 MCP 客户端。

### npm

```bash
codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp
```

npm 包 `cmsis-dap-mcp` 会自动下载对应平台的二进制
（win32/linux/darwin × x64/arm64）。

## AI 客户端配置

服务器通过 stdio 使用 MCP 协议。以下是经过实测的 Codex、Claude Code、
opencode 配置。请把 `/path/to/cmsis-dap-mcp` 换成你的二进制路径，或使用
`npx -y cmsis-dap-mcp`。

### Codex

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

### Claude Code

```bash
claude mcp add --scope local cmsis-dap -- /path/to/cmsis-dap-mcp
```

用 `claude mcp list` 确认（显示 `√ Connected`）。

### opencode

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

### 其他 MCP 客户端

任何兼容 MCP 的客户端都可以使用 stdio 服务器：

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

## 快速开始（已实测）

以下流程已在 CMSIS-DAP 探针 + Cortex-M0+ 开发板上端到端验证，并通过
Claude Code、opencode 和原始 MCP stdio 驱动：

1. `list_probes` 查找探针 id。
2. `connect`，参数 `{"protocol": "swd", "speed_khz": 1000}`。
3. `read_memory` / `write_memory` 原始内存访问。
4. `halt`，然后 `read_core_register`（例如 `pc`、`sp`、`lr`、`r0`）。
5. 完成后 `resume`。
6. `load_svd` 加载你自己的 SVD 文件，进行命名外设访问。
7. 只有以 `--allow-destructive` 启动服务器后才可 `program_flash`。

示例会话（真实输出）：

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

## 使用 SVD 文件

```text
load_svd { "path": "/path/to/your-chip.svd" }
list_peripherals {}
read_peripheral { "peripheral": "GPIOA", "register": "ODR" }
write_peripheral { "peripheral": "GPIOA", "register": "ODR", "field": "ODR0", "value": 1 }
```

SVD 文件由用户运行时提供；本仓库不捆绑芯片专有数据。

## Flash 编程

Flash 工具需要带烧写算法的目标描述。启动时提供 probe-rs 目标描述 YAML：

```bash
cmsis-dap-mcp --target-yaml /path/to/your-target.yaml --allow-destructive
```

然后用 YAML 中的目标名连接并编程：

```text
connect { "protocol": "swd", "target": "YourChip" }
erase_flash { "address": 0x08000000, "size": 0x1000 }
program_flash { "address": 0x08000000, "data": [0x00, 0x11, ...], "verify": true }
```

`verify: true` 会在烧写后读回校验。`erase_flash` 只擦除与请求范围重叠的扇区。

## 安全

- 只读工具始终可用。
- 写与调试控制工具标记为写操作，由你的 MCP 客户端审批策略决定。
- `erase_flash` 与 `program_flash` 为破坏性工具，默认禁用，仅当以
  `--allow-destructive` 启动时可用。

Flash 擦除、Option 字节修改、读保护与调试解锁可能导致设备永久损坏或不可恢复。
只有明确要重新编程目标时才启用破坏性模式。

## Linux udev

在 Linux 上，为当前用户授予调试探针访问权限（一次性）：

```text
# 以 CMSIS-DAP v1/v2 探针为例；请按你的硬件调整 VID/PID
SUBSYSTEM=="usb", ATTRS{idVendor}=="xxxx", ATTRS{idProduct}=="yyyy", MODE="0666"
```

## 开发

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
mdbook build docs        # 英文文档
mdbook build docs/zh     # 中文文档
```

## 文档

完整文档（英文与中文）发布在 GitHub Pages：

- 英文：<https://guohj2021.github.io/CMSIS-DAP-MCP/>
- 中文：<https://guohj2021.github.io/CMSIS-DAP-MCP/zh/>

## 许可证

MIT OR Apache-2.0
