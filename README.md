# CMSIS-DAP MCP

> CMSIS-DAP tools for Cortex-M: an MCP server for AI assistants plus a
> standalone CLI, both built on the same engine.

[English](#english) · [中文](#chinese)

![License](https://img.shields.io/github/license/guohj2021/CMSIS-DAP-MCP)
![CI](https://img.shields.io/github/actions/workflow/status/guohj2021/CMSIS-DAP-MCP/ci.yml?branch=main&label=CI)
<!-- Keep the Release badge ?branch= in sync with the latest release tag; the
     tag-triggered Release workflow never runs on main. -->
![Release](https://img.shields.io/github/actions/workflow/status/guohj2021/CMSIS-DAP-MCP/release.yml?branch=v0.4.1&label=Release)
![Pages](https://img.shields.io/github/actions/workflow/status/guohj2021/CMSIS-DAP-MCP/pages.yml?branch=main&label=Pages)
![Version](https://img.shields.io/github/v/tag/guohj2021/CMSIS-DAP-MCP)
![Rust](https://img.shields.io/badge/rust-1.97.1-orange)
![npm](https://img.shields.io/npm/v/cmsis-dap-mcp)
![Platform](https://img.shields.io/badge/platform-win32%20%7C%20linux%20%7C%20macos-lightgrey)
![Docs](https://img.shields.io/badge/docs-mdBook-blue)

---

<a id="english"></a>
# English

## What is it?

**CMSIS-DAP MCP** is a repository with two tools built on the same engine:

- **cmsis-dap-mcp** — a [Model Context Protocol](https://modelcontextprotocol.io)
  server that exposes your CMSIS-DAP debug probe to AI assistants;
- **cmsis-dap-cli** — a standalone command-line tool for humans, scripts and
  automation, with the same capabilities and no AI client needed.

Both enumerate probes, connect over SWD / JTAG, read/write memory and core
registers, control execution, use named peripherals via SVD files, program
flash from firmware files, and run J-Link / OpenOCD style debug scripts.

- Generic Cortex-M support: standard cores work without chip-specific
  adaptation.
- Named peripheral access: load any CMSIS-SVD file at runtime; chip files are
  never bundled.
- Flash programming from `axf` / `elf` / `bin` / `hex` files, plus `bin` /
  `hex` memory export.
- Zero runtime dependencies for end users: `npx -y cmsis-dap-mcp` or one
  native binary (`npx -y cmsis-dap-cli` for the CLI).
- Cross-platform: Windows / Linux / macOS.

## Features

| Area | Tools |
| --- | --- |
| Probe | `list_probes`, `get_probe_info`, `connect`, `disconnect`, `get_target_info` |
| Memory | `read_memory`, `write_memory`, `verify_memory`, memory export (`bin`/`hex`) |
| Core | `read_core_register`, `write_core_register`, `list_core_registers`, `get_core_status`, `halt`, `resume`, `step`, `reset` |
| Breakpoints | `set_breakpoint`, `clear_breakpoints`, `list_breakpoints` |
| Watchpoints | `set_watchpoint`, `clear_watchpoints`, `list_watchpoints` |
| DAP | `read_dap`, `write_dap` |
| SVD | `load_svd`, `list_peripherals`, `read_peripheral`, `write_peripheral` |
| Files | `program_flash` (`axf`/`elf`/`bin`/`hex`), `read_memory` export |
| Scripts | `run_script` (J-Link / OpenOCD style) |
| Flash | `erase_flash`, `program_flash` |

The CLI mirrors these capabilities as subcommands (`read`, `write`, `reg`,
`halt`, `flash program`, `svd read`, ...); see the [CLI section](#cli).
The CLI also ships live debugging features that are not part of the MCP
server: `watch` (live variable polling), `rtt monitor` (SEGGER RTT logs) and
`evr monitor` (CMSIS-View Event Recorder) all run over SWD/JTAG — no UART —
with timestamped log export.

## Architecture

The server is a pure MCP stdio process: an AI client drives it, a `Backend`
abstraction sits between the tool layer and probe-rs (with a mock backend for
tests), and logs never touch stdout. See the
[Architecture](https://guohj2021.github.io/CMSIS-DAP-MCP/architecture.html)
page for module responsibilities and data flow.

The repository is a Cargo workspace: `cmsis-dap-core` (shared engine),
`cmsis-dap-mcp` (the MCP server binary) and `cmsis-dap-cli` (the CLI binary).

![CMSIS-DAP MCP architecture](docs/src/images/architecture.png)

## Quickstart (npm, recommended)

Install nothing. Configure your MCP client to launch the server with `npx`:

```bash
codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp
```

Or just tell your AI assistant: "install the CMSIS-DAP MCP server launched
with `npx -y cmsis-dap-mcp`" — it configures and verifies the server for you,
no manual configuration needed.

The `cmsis-dap-mcp` npm package automatically downloads the correct platform
binary (win32/linux/darwin × x64/arm64).

Or use a native binary from [GitHub Releases](https://github.com/guohj2021/CMSIS-DAP-MCP/releases).

## AI client configuration

The server speaks MCP over stdio. Below are the standard configurations for
Codex, Claude Code and opencode. All examples use `npx`; to run a local build
instead, replace `npx -y cmsis-dap-mcp` with your binary path (see
[Configuration styles](#configuration-styles)).

### Codex

```bash
codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp
```

Or add to `~/.codex/config.toml`:

```toml
[mcp_servers.cmsis-dap]
command = "npx"
args = ["-y", "cmsis-dap-mcp"]
```

Verify with `codex mcp list`. The Codex desktop app loads the server when a
new session starts.

### Claude Code

```bash
claude mcp add --scope local cmsis-dap -- npx -y cmsis-dap-mcp
```

Verify with `claude mcp list` (shows `√ Connected`).

### opencode

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

Verify with `opencode mcp list`.

### Other MCP clients

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

## Configuration styles

There are three ways to point an MCP client at a server. They are all
standard; pick the one that fits your situation.

| Style | Example | Best for |
| --- | --- | --- |
| `npx` package (standard) | `command = "npx", args = ["-y", "cmsis-dap-mcp"]` | Published releases; updates with npm; no manual file management |
| Local binary | `command = "/path/to/cmsis-dap-mcp"` | Unpublished builds, private/offline use, exact version control |
| Remote URL | `url = "https://..."` | Streamable-HTTP MCP servers (not supported by this project yet) |

`npx` fetches the published package on first launch and caches it afterwards.
To pin a version, use `npx -y cmsis-dap-mcp@0.4.0`. To run the freshly built
local binary instead (for example while developing this repository), point
the client at `target/release/cmsis-dap-mcp` — no npm publish needed.

## Usage examples

Typical session (verified on hardware):

```text
list_probes -> {"probes": [{"id": "0123456789AB", "product": "XV-Link CMSIS-DAP", ...}]}
connect {protocol: swd, speed_khz: 1000}
  -> {"target": {"core_type": "Armv6m", "core_count": 1, "ap_count": 1, ...}}
read_memory {address: 0x20000000, width: u32, count: 4}
  -> {"values": [64000000, 1, 3, 0]}
halt -> {"halted": true}
read_core_register {name: pc} -> {"value": 134228884}
resume -> {"running": true}
```

Program a firmware file and export memory:

```text
program_flash { "address": 0x08004000, "path": "fw.hex", "format": "hex", "verify": true }
read_memory   { "address": 0x08000000, "width": "u8", "count": 0x1000, "path": "fw.bin", "format": "bin" }
```

Run a J-Link / OpenOCD style script:

```text
run_script { "script": "connect\nhalt\nreg pc\nsavebin C:/dump.bin 0x20000000 0x100\nresume" }
```

## CLI

Besides the MCP server, the project ships a standalone `cmsis-dap-cli` that
shares the same engine. A typical session:

```bash
cmsis-dap-cli chip search STM32F030          # find a built-in chip
cmsis-dap-cli --target STM32F030C8 connect   # connect (or --target-yaml for custom chips)
cmsis-dap-cli --target STM32F030C8 read --address 0x20000000 --width u32 --count 4
cmsis-dap-cli --target STM32F030C8 halt
cmsis-dap-cli --target STM32F030C8 reg get pc
cmsis-dap-cli --target STM32F030C8 flash program --address 0x08000000 --file fw.hex --verify
cmsis-dap-cli --target STM32F030C8 --elf fw.axf watch counter --interval-ms 200 --count 0
cmsis-dap-cli --target STM32F030C8 --elf fw.axf rtt monitor --channel 0 --count 0
cmsis-dap-cli --target STM32F030C8 --elf fw.axf evr monitor --ctx 0,2 --count 0
cmsis-dap-cli --target STM32F030C8 repl
```

`--json` gives machine-readable output; flash erase/program run directly. The
npm package (`npx -y cmsis-dap-cli`) ships with each release; for local builds
use `target/release/cmsis-dap-cli`. `--elf` adds symbol names for `watch` /
RTT / Event Recorder; monitor commands print timestamped lines and export the
same log to the current directory by default (`--log-dir` / `--log-file` to
choose the location). See the
[CLI documentation](./docs/src/cli.md) for the full command reference.

## Live debugging (watch / RTT / Event Recorder)

Three CLI-only features read live data from the target over SWD/JTAG — no
UART and no trace hardware needed:

| Command | What it does |
| --- | --- |
| `watch TARGET...` | poll variables by address or ELF symbol with a configurable refresh interval (`--interval-ms`, `--width`) |
| `rtt monitor --channel 0,1` | read SEGGER RTT up-channel logs (firmware must call `SEGGER_RTT_Init`) |
| `evr monitor` | decode CMSIS-View Event Recorder events (firmware must include the Event Recorder component) |

Symbols come from the firmware ELF (`--elf`); the RTT control block and
`EventRecorderInfo` are located automatically from it. Every monitor line
carries a host timestamp `[YYYY-MM-DD HH:MM:SS.mmm]` and is exported to a log
file in the current directory by default (`--log-dir` / `--log-file` to
choose; `--json` emits NDJSON with a `host_ts` field). `--count N` bounds the
run for scripts and CI; `--count 0` runs until Ctrl-C.

```bash
cmsis-dap-cli --target STM32F030C8 --elf fw.axf watch counter --interval-ms 200 --count 0
cmsis-dap-cli --target STM32F030C8 --elf fw.axf rtt monitor --channel 0 --count 0 --log-dir logs
cmsis-dap-cli --target STM32F030C8 --elf fw.axf evr monitor --ctx 0,2 --count 0
```

Example output, verified on a CMSIS-DAP probe with a Cortex-M0+ target running
SEGGER RTT and the Event Recorder:

```text
[2026-08-16 19:16:13.302] watch_var = 0x00001007
[2026-08-16 19:16:13.520] counter = 0x00000001
[2026-08-16 19:16:14.268] [RTT0 "Terminal"] Live test: RTT up, core 8 MHz
[2026-08-16 19:16:15.727] evr ticks=31688 t=0.003961s ctx=0x0 comp=0xAB msg=0x02 seq=8 val1=0x00000001 val2=0x00001007 first last
```

In `repl` use `connect` + `reset run` first (a fresh session attaches with the
core halted), then `watch run` / `rtt monitor` / `evr monitor`.

## Security

- Read-only tools are always available.
- Write and debug-control tools are marked as writes; your MCP client governs
  approval.
- `erase_flash` and `program_flash` are destructive and disabled unless the
  server is started with `--allow-destructive`. Destructive script commands
  require the same flag.
- Memory export and scripts read/write files on the host at paths you
  provide (same trust model as `load_svd`).

Flash erasing, option-byte changes, read-protection and debug unlock can
permanently damage a device or make it unrecoverable.

## Documentation

Full documentation (English and Chinese) is published on GitHub Pages:

- English: <https://guohj2021.github.io/CMSIS-DAP-MCP/>
- 中文: <https://guohj2021.github.io/CMSIS-DAP-MCP/zh/>

## Development

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
scripts/check-no-vendor.ps1   # Windows PowerShell; no vendor-specific terms
mdbook build docs        # English docs
mdbook build docs/zh     # Chinese docs
```

Releases: pushing a `v*` tag runs the Release workflow, which builds the three
platform binaries, publishes the npm packages (`cmsis-dap-mcp` / `cmsis-dap-cli`
and the platform packages) and creates the GitHub Release.

## License

MIT OR Apache-2.0

---

<a id="chinese"></a>
# 中文

## 这是什么

**CMSIS-DAP MCP** 是一个包含两个工具的仓库，两者共用同一套引擎：

- **cmsis-dap-mcp** —— 一个 [模型上下文协议（MCP）](https://modelcontextprotocol.io)
  服务器，把 CMSIS-DAP 调试探针开放给 AI 助手；
- **cmsis-dap-cli** —— 面向人、脚本与自动化的独立命令行工具，能力一致，
  无需 AI 客户端。

两者都可以枚举探针、通过 SWD 或 JTAG 连接、读写内存与内核寄存器、控制执行、
用 SVD 做命名外设访问、从固件文件烧录 Flash、导出内存，以及运行 J-Link /
OpenOCD 风格调试脚本。

- 通用 Cortex-M 支持：标准内核无需芯片适配。
- 命名外设访问：运行时加载任意 CMSIS-SVD 文件；仓库不捆绑芯片文件。
- 支持 `axf`/`elf`/`bin`/`hex` 固件烧录，以及 `bin`/`hex` 内存导出。
- 终端用户零安装：`npx -y cmsis-dap-mcp` 或单个原生二进制。
- 跨平台：Windows / Linux / macOS。

## 功能

| 分类 | 工具 |
| --- | --- |
| 探针 | `list_probes`、`get_probe_info`、`connect`、`disconnect`、`get_target_info` |
| 内存 | `read_memory`、`write_memory`、`verify_memory`、内存导出（`bin`/`hex`） |
| 内核 | `read_core_register`、`write_core_register`、`list_core_registers`、`get_core_status`、`halt`、`resume`、`step`、`reset` |
| 断点 | `set_breakpoint`、`clear_breakpoints`、`list_breakpoints` |
| 数据观察点 | `set_watchpoint`、`clear_watchpoints`、`list_watchpoints` |
| DAP | `read_dap`、`write_dap` |
| SVD | `load_svd`、`list_peripherals`、`read_peripheral`、`write_peripheral` |
| 文件 | `program_flash`（`axf`/`elf`/`bin`/`hex`）、`read_memory` 导出 |
| 脚本 | `run_script`（J-Link / OpenOCD 风格） |
| Flash | `erase_flash`、`program_flash` |

CLI 以子命令形式提供相同能力（`read`、`write`、`reg`、`halt`、`flash program`、
`svd read` 等），见[命令行工具](#命令行工具)。
CLI 还提供 MCP 服务器没有的实时调试能力：`watch`（变量实时轮询）、`rtt
monitor`（SEGGER RTT 日志）与 `evr monitor`（CMSIS-View Event Recorder）都
走 SWD/JTAG——无需串口——并支持带时间戳的日志导出。

## 架构

服务器是纯 MCP stdio 进程：由 AI 客户端驱动，工具层与 probe-rs 之间隔着一层
`Backend` 抽象（另有 Mock 后端用于测试），日志不写 stdout。模块职责与数据流
见[架构说明](https://guohj2021.github.io/CMSIS-DAP-MCP/zh/architecture.html)
页面。

仓库采用 Cargo workspace：`cmsis-dap-core`（共享引擎）、`cmsis-dap-mcp`
（MCP 服务器二进制）与 `cmsis-dap-cli`（命令行工具二进制）。

![CMSIS-DAP MCP 架构图](docs/src/images/architecture.png)

## 快速开始（npm，推荐）

无需安装：用 `npx` 让客户端启动服务器即可：

```bash
codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp
```

也可以直接告诉 AI 助手：“帮我安装 CMSIS-DAP MCP 服务器，用
`npx -y cmsis-dap-mcp` 启动”——它会替你完成配置并验证，无需手动改配置。

`cmsis-dap-mcp` npm 包会自动下载对应平台的二进制
（win32/linux/darwin × x64/arm64）。

也可以从 [GitHub Releases](https://github.com/guohj2021/CMSIS-DAP-MCP/releases)
下载原生二进制。

## AI 客户端配置

服务器通过 stdio 使用 MCP 协议。以下是 Codex、Claude Code、opencode 的标准
配置，全部以 `npx` 为例；要使用本地构建，把 `npx -y cmsis-dap-mcp` 换成
二进制路径即可（见[配置方式](#配置方式)）。

### Codex

```bash
codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp
```

或写入 `~/.codex/config.toml`：

```toml
[mcp_servers.cmsis-dap]
command = "npx"
args = ["-y", "cmsis-dap-mcp"]
```

用 `codex mcp list` 确认。Codex 桌面端会在新会话启动时加载该服务器。

### Claude Code

```bash
claude mcp add --scope local cmsis-dap -- npx -y cmsis-dap-mcp
```

用 `claude mcp list` 确认（显示 `√ Connected`）。

### opencode

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

用 `opencode mcp list` 确认。

### 其他 MCP 客户端

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

## 配置方式

把 MCP 客户端指向服务器有三种写法，都是标准做法，按场景选择：

| 方式 | 示例 | 适用场景 |
| --- | --- | --- |
| `npx` 包（标准） | `command = "npx", args = ["-y", "cmsis-dap-mcp"]` | 已发布版本；随 npm 更新，无需管理文件 |
| 本地二进制 | `command = "/path/to/cmsis-dap-mcp"` | 未发布构建、私有/离线使用、精确版本控制 |
| 远程 URL | `url = "https://..."` | Streamable-HTTP MCP 服务器（本项目暂不支持） |

`npx` 首次启动时下载已发布包并缓存。要固定版本，用
`npx -y cmsis-dap-mcp@0.4.0`。要运行刚构建的本地二进制（例如开发本仓库时），
把客户端指向 `target/release/cmsis-dap-mcp` 即可，无需发布 npm。

## 使用示例

典型会话（已实测）：

```text
list_probes -> {"probes": [{"id": "0123456789AB", "product": "XV-Link CMSIS-DAP", ...}]}
connect {protocol: swd, speed_khz: 1000}
  -> {"target": {"core_type": "Armv6m", "core_count": 1, "ap_count": 1, ...}}
read_memory {address: 0x20000000, width: u32, count: 4}
  -> {"values": [64000000, 1, 3, 0]}
halt -> {"halted": true}
read_core_register {name: pc} -> {"value": 134228884}
resume -> {"running": true}
```

烧录固件文件并导出内存：

```text
program_flash { "address": 0x08004000, "path": "fw.hex", "format": "hex", "verify": true }
read_memory   { "address": 0x08000000, "width": "u8", "count": 0x1000, "path": "fw.bin", "format": "bin" }
```

运行 J-Link / OpenOCD 风格脚本：

```text
run_script { "script": "connect\nhalt\nreg pc\nsavebin C:/dump.bin 0x20000000 0x100\nresume" }
```

## 命令行工具

除 MCP 服务器外，项目还提供独立的 `cmsis-dap-cli`，与服务器共用同一套引擎：

```bash
cmsis-dap-cli chip search STM32F030          # 查找内置芯片
cmsis-dap-cli --target STM32F030C8 connect   # 连接（自定义芯片用 --target-yaml）
cmsis-dap-cli --target STM32F030C8 read --address 0x20000000 --width u32 --count 4
cmsis-dap-cli --target STM32F030C8 halt
cmsis-dap-cli --target STM32F030C8 reg get pc
cmsis-dap-cli --target STM32F030C8 flash program --address 0x08000000 --file fw.hex --verify
cmsis-dap-cli --target STM32F030C8 --elf fw.axf watch counter --interval-ms 200 --count 0
cmsis-dap-cli --target STM32F030C8 --elf fw.axf rtt monitor --channel 0 --count 0
cmsis-dap-cli --target STM32F030C8 --elf fw.axf evr monitor --ctx 0,2 --count 0
cmsis-dap-cli --target STM32F030C8 repl
```

`--json` 输出机器可读结果；Flash 擦除/烧录直接执行。npm 包
（`npx -y cmsis-dap-cli`）随每次发布提供；本地构建用
`target/release/cmsis-dap-cli`。`--elf` 为 `watch`/RTT/Event Recorder 提供
符号名；监控命令打印带时间戳的行，并默认把同样内容导出到当前目录的日志
（`--log-dir`/`--log-file` 指定位置）。完整命令参考见
[命令行工具文档](./docs/zh/src/cli.md)。

## 实时调试（watch / RTT / Event Recorder）

以下三项 CLI 独有能力全部走 SWD/JTAG 实时读取目标数据——无需串口、无需
trace 硬件：

| 命令 | 功能 |
| --- | --- |
| `watch TARGET...` | 按地址或 ELF 符号轮询变量，可配置刷新间隔（`--interval-ms`、`--width`） |
| `rtt monitor --channel 0,1` | 读取 SEGGER RTT 上行通道日志（固件需调用 `SEGGER_RTT_Init`） |
| `evr monitor` | 解码 CMSIS-View Event Recorder 事件（固件需包含 Event Recorder 组件） |

符号来自固件 ELF（`--elf`），RTT 控制块与 `EventRecorderInfo` 会自动从其中
定位。每条监控输出都带主机时间戳 `[YYYY-MM-DD HH:MM:SS.mmm]`，并默认导出到
当前目录的日志文件（`--log-dir`/`--log-file` 指定位置；`--json` 输出带
`host_ts` 字段的 NDJSON）。`--count N` 限定轮数便于脚本与 CI；`--count 0`
一直运行到 Ctrl-C。

```bash
cmsis-dap-cli --target STM32F030C8 --elf fw.axf watch counter --interval-ms 200 --count 0
cmsis-dap-cli --target STM32F030C8 --elf fw.axf rtt monitor --channel 0 --count 0 --log-dir logs
cmsis-dap-cli --target STM32F030C8 --elf fw.axf evr monitor --ctx 0,2 --count 0
```

实测输出示例（CMSIS-DAP 探针 + 运行 SEGGER RTT 与 Event Recorder 的
Cortex-M0+ 目标）：

```text
[2026-08-16 19:16:13.302] watch_var = 0x00001007
[2026-08-16 19:16:13.520] counter = 0x00000001
[2026-08-16 19:16:14.268] [RTT0 "Terminal"] Live test: RTT up, core 8 MHz
[2026-08-16 19:16:15.727] evr ticks=31688 t=0.003961s ctx=0x0 comp=0xAB msg=0x02 seq=8 val1=0x00000001 val2=0x00001007 first last
```

在 `repl` 里先 `connect` + `reset run`（新会话附着时核心处于停机），再执行
`watch run` / `rtt monitor` / `evr monitor`。

## 安全

- 只读工具始终可用。
- 写与调试控制工具标记为写操作，由 MCP 客户端审批。
- `erase_flash` 与 `program_flash` 为破坏性工具，默认禁用，仅当以
  `--allow-destructive` 启动时可用；脚本中的破坏性命令同样需要该开关。
- 内存导出与脚本会在你提供的路径读写主机文件（与 `load_svd` 相同的信任
  模型）。

Flash 擦除、Option 字节修改、读保护与调试解锁可能导致设备永久损坏或不可恢复。

## 文档

完整文档（英文与中文）发布在 GitHub Pages：

- 英文：<https://guohj2021.github.io/CMSIS-DAP-MCP/>
- 中文：<https://guohj2021.github.io/CMSIS-DAP-MCP/zh/>

## 开发

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
scripts/check-no-vendor.ps1   # Windows PowerShell；不得出现厂商专有词
mdbook build docs        # 英文文档
mdbook build docs/zh     # 中文文档
```

发布：推送 `v*` tag 会触发 Release 流程，构建三平台二进制、发布 npm 包
（`cmsis-dap-mcp` / `cmsis-dap-cli` 及平台包）并创建 GitHub Release。

## 许可证

MIT OR Apache-2.0
