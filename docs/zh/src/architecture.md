# 架构说明

`cmsis-dap-mcp` 是单个 Rust 进程，通过 stdio 使用 MCP 协议。它是纯服务器：
由 MCP 客户端（Codex、Claude Code、opencode 或任意兼容 MCP 的主机）驱动，
自身不提供界面。

## 系统总览

![架构总览](images/architecture.png)

```text
MCP 客户端（Codex / Claude Code / opencode / 任意 MCP 主机）
    |
    |  MCP stdio：JSON-RPC 2.0，换行分隔，运行在 stdout
    v
+--------------------------------------------------------------+
|  cmsis-dap-mcp（单个 Rust 进程，日志只写 stderr）              |
|                                                              |
|  +--------------------------------------------------------+  |
|  | MCP 工具层（rmcp）                                       |  |
|  |  probe | memory | core | dap | svd | flash | file | script | |
|  +--------------------------------------------------------+  |
|  | 安全策略：只读 / 写 / 破坏性                              |  |
|  +--------------------------------------------------------+  |
|  | 会话管理：探针选择、会话与 SVD 状态                       |  |
|  +--------------------------------------------------------+  |
|  | 后端接口（Backend trait）                                |  |
|  |  ProbeRsBackend（真实）         MockBackend（测试）      |  |
|  +--------------------------------------------------------+  |
|  | probe-rs 库（SWD/JTAG、Flash、ELF/HEX/BIN 解析）          |  |
|  +--------------------------------------------------------+  |
+--------------------------------------------------------------+
    |
    |  USB（HID / WinUSB）
    v
CMSIS-DAP 探针 ---- SWD / JTAG ----> Cortex-M 目标
```

## 模块职责

| 模块 | 职责 |
| --- | --- |
| `cli` | 解析启动参数、配置日志、启动 stdio 服务器 |
| `mcp` | 用 rmcp 注册工具、MCP 注解、server instructions |
| `mcp/tools_*` | 各领域参数与处理器（probe、memory、core、dap、svd、flash、script） |
| `script` | 线性 J-Link Commander / OpenOCD 风格脚本解析与执行 |
| `hex` | 内存导出用的 Intel HEX 编码器 |
| `security` | 三级策略；破坏性工具需要 `--allow-destructive` |
| `session` | 单个活动会话；持有探针/会话与 SVD 状态 |
| `backend` | `Backend` trait 及 `ProbeRsBackend`、`MockBackend` 实现 |
| `svd` | SVD 解析与外设/寄存器/位域命名解析 |
| `error` | 错误码与结构化 `McpError` |

## 工具调用流程

```text
MCP 客户端          服务器                  后端                目标
   |  tools/call      |                        |                      |
   |----------------->|  安全检查               |                      |
   |                  |  锁定会话               |                      |
   |                  |  backend.read_memory() |-- SWD/JTAG 读取 ---->|
   |                  |<-----------------------|                      |
   |<-----------------|  结构化 JSON            |                      |
```

每次工具调用都走同一条路径：解析并校验参数 → 检查安全等级 → 获取会话 →
在后端执行操作 → 返回结构化 JSON（或分类错误）。

## 文件与脚本路径

```text
program_flash {data: [...]}  ->  backend.program_flash  ->  FlashLoader（原始数据）
program_flash {path, format} ->  backend.program_file   ->  BIN：读取 + add_data
                                                             ELF/AXF/HEX：probe-rs build_loader
read_memory {path, format}   ->  backend.export_memory  ->  BIN：原始字节
                                                             HEX：hex::encode_ihex
run_script {path | script}   ->  script::run           ->  逐命令分派到后端
                                                             （破坏性命令由策略门禁）
```

## 构建与发布流程

```text
特性分支 -> develop -> main -> tag vX.Y.Z
                                  |
                                  v
             CI：三平台 fmt / clippy / test / build
                                  |
             +--------------------+--------------------+
             |                                         |
             v                                         v
 GitHub Release 二进制                        npm 平台包
 （win32-x64 / linux-x64 / darwin-x64）        （元包 cmsis-dap-mcp + 平台包）
             |
             v
 GitHub Pages 文档（英文 /，中文 /zh/）
```
