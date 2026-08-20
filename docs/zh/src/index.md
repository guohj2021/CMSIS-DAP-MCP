# CMSIS-DAP MCP

面向 **CMSIS-DAP** 调试探针与 **Cortex-M** 芯片的开源调试工具套件，提供
MCP 服务器（供 AI 助手驱动）和独立命令行工具，两者共用同一引擎，通过
**SWD** 或 **JTAG** 工作。

**新用户？** 请从[从零开始](./getting-started.md)入手，一步步完成环境搭建
和首次连接。

## 两个工具

- **cmsis-dap-mcp** —— MCP（模型上下文协议）服务器，让 AI 助手（Codex、
  Claude Code、opencode 等）直接操控探针和目标芯片。
- **cmsis-dap-cli** —— 面向人、脚本与自动化的独立命令行工具，无需 AI 客户端。

## 核心功能

### 探针与会话

| 工具 | 功能 |
| --- | --- |
| `list_probes` | 枚举所有已连接的 CMSIS-DAP 探针 |
| `get_probe_info` | 查看探针详细信息（型号、序列号、支持的协议和速度） |
| `connect` | 通过 SWD 或 JTAG 连接目标芯片，支持按住复位连接 |
| `disconnect` | 断开当前会话 |
| `get_target_info` | 查看目标信息（内核类型、CPUID、内存映射） |

### 内存访问

| 工具 | 功能 |
| --- | --- |
| `read_memory` | 读取内存，支持 u8/u16/u32/u64，可导出为 bin/hex 文件 |
| `write_memory` | 写入内存 |
| `verify_memory` | 读回并与期望值比较，报告不匹配项 |

### 内核控制

| 工具 | 功能 |
| --- | --- |
| `read_core_register` / `write_core_register` | 读写内核寄存器（pc、sp、lr、r0-r15 等） |
| `list_core_registers` | 列出目标支持的全部寄存器 |
| `get_core_status` | 查看内核状态（运行/停机/睡眠/锁定） |
| `halt` / `resume` / `step` | 暂停 / 恢复 / 单步执行 |
| `reset` | 复位目标，支持复位后继续或复位后暂停 |

### 断点与数据观察点

| 工具 | 功能 |
| --- | --- |
| `set_breakpoint` / `clear_breakpoints` / `list_breakpoints` | 硬件断点管理 |
| `set_watchpoint` / `clear_watchpoints` / `list_watchpoints` | DWT 数据观察点（读/写/读写触发） |

### DAP 原始访问

| 工具 | 功能 |
| --- | --- |
| `read_dap` / `write_dap` | 直接读写 DP/AP 寄存器（高级调试） |

### SVD 命名外设

| 工具 | 功能 |
| --- | --- |
| `load_svd` | 运行时加载任意 CMSIS-SVD 文件 |
| `list_peripherals` | 列出所有已加载的外设 |
| `read_peripheral` / `write_peripheral` | 按名称读写外设寄存器和位域（读-改-写） |

### Flash 烧录

| 工具 | 功能 |
| --- | --- |
| `erase_flash` | 按扇区擦除 Flash（只擦与请求范围重叠的扇区） |
| `program_flash` | 烧录固件，支持 elf/axf/bin/hex 格式，可选读回校验 |

### 芯片定义

| 工具 | 功能 |
| --- | --- |
| `define_chip`（MCP） | 从 Keil FLM 文件运行时注册未知芯片，无需外部工具 |
| `chip generate`（CLI） | 从 FLM 生成 probe-rs target YAML 文件 |
| `chip list` / `chip search` | 列出或搜索内置芯片库 |

### 脚本引擎

| 工具 | 功能 |
| --- | --- |
| `run_script`（MCP）/ `script`（CLI） | 运行 J-Link Commander / OpenOCD 风格调试脚本 |

### 非侵入调试

| 工具 | 功能 |
| --- | --- |
| `dump_cpu_state`（MCP）/ `dump`（CLI） | 不复位目标，采集 CPU 快照（寄存器、fault 状态、栈、内存） |

### 远程访问

| 功能 | 说明 |
| --- | --- |
| TCP JSON-RPC 服务器 | `--tcp PORT`（MCP）或 `tcp-server`（CLI），按行分隔的远程协议 |
| GDB 服务器 | `--gdb-port PORT`（MCP）或 `gdb-server`（CLI），GDB Remote Serial Protocol stub |

### 运行时配置

| 工具 | 功能 |
| --- | --- |
| `get_config` | 查看当前运行时配置 |
| `update_config` | 运行时更新配置（破坏性开关、TCP/GDB 端口），无需重启 |
| `reload_config` | 重新加载启动时指定的配置文件 |

### 安全

三级安全策略：只读工具始终可用；写工具由 MCP 客户端审批；破坏性工具
（Flash 擦除/烧录）默认禁用，需显式启用。

## CLI 实时调试（独有）

| 功能 | 说明 |
| --- | --- |
 | `watch` | 按刷新间隔轮询变量（地址或 ELF 符号），带时间戳日志导出 |
 | `rtt monitor` | 读取 SEGGER RTT 上行通道日志，无需串口 |
 | `evr monitor` | 解码 CMSIS-View Event Recorder 事件，无需 trace 硬件 |
 | `repl` | 交互式 shell，保持会话持续操作 |

## 亮点特性

- **通用 Cortex-M 支持**：标准内核无需芯片适配即可调试
- **运行时芯片定义**：从 FLM 文件注册未知芯片，无需预构建 YAML
- **零参数启动**：服务器可空启动，运行时通过工具完全配置
- **终端用户零依赖**：`npx -y cmsis-dap-mcp` 或单个原生二进制
- **跨平台**：Windows / Linux / macOS

## 文档导航

- [从零开始](./getting-started.md) —— 完整的环境搭建与首次连接教程
- [快速开始](./quickstart.md) —— MCP 服务器快速配置
- [AI 客户端配置](./ai-clients.md) —— Codex / Claude Code / opencode 配置
- [工具参考](./tools.md) —— MCP 工具完整参考
- [命令行工具](./cli.md) —— CLI 命令完整参考
- [脚本使用](./scripting.md) —— J-Link / OpenOCD 风格脚本
- [SWD 与 JTAG](./swd-jtag.md) —— 协议选择
- [SVD 与 Flash](./svd-flash.md) —— 外设访问与烧录
- [安全](./security.md) —— 安全模型与配置
- [故障排查](./troubleshooting.md) —— 常见问题

英文文档：<https://guohj2021.github.io/CMSIS-DAP-MCP/>
