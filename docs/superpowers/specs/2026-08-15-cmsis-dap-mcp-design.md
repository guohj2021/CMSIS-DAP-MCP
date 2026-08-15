# CMSIS-DAP MCP 设计规格

| 项 | 值 |
| --- | --- |
| 主题 | 通过 MCP 让 AI 直接操作 CMSIS-DAP 调试器访问 Cortex-M 芯片内部资源 |
| 日期 | 2026-08-15 |
| 状态 | 架构 v1 已获用户批准 |
| 交付物 | 独立 MCP 服务器（Rust）+ npm 自动安装入口 + 三平台 Release + GitHub Pages 文档 |

## 1. 目标与成功标准

构建一个通用、无厂商绑定、用户零依赖的 CMSIS-DAP MCP 服务器，使 AI 客户端能够枚举调试探针、连接目标芯片、读写内存与外设、控制核心执行并（可选）执行 Flash 编程。

成功标准：

- 用户按标准 MCP 配置即可使用（`npx -y ...` 或原生可执行文件），无需安装 Python、OpenOCD、Node.js、Rust 或任何调试工具链。
- 任意标准 Cortex-M 芯片（含 Cortex-M0）可连接并执行原始内存/外设访问与核心调试，无需芯片适配。
- 提供 SVD 后可进行命名外设访问；提供 CMSIS-Pack/FLM 或等价 target 描述后可进行 Flash 擦写。
- 默认安全：只读操作放行，写/调试控制按客户端策略确认，破坏性操作默认禁用并需显式 `--allow-destructive`。
- 三平台（Windows / Linux / macOS）CI 构建与测试，GitHub Release 发布，GitHub Pages 文档。
- GitFlow：`feature -> develop -> main`，本地 ALB32F033C8 实机验证通过后才推送与发布。
- 公开仓库不包含 ALB32 专有内容，也不绑定任何具体芯片。

## 2. 范围与非目标

范围内：

- CMSIS-DAP v1（HID）与 v2（WinUSB）探针；SWD 为主、JTAG 可用。
- ADIv5 DP/AP 访问、CoreSight 组件枚举、Cortex-M 内存与核心寄存器访问。
- 暂停、运行、单步、断点、复位。
- SVD 解析与命名外设/寄存器/位域访问。
- 基于 CMSIS-Pack Flash 算法（经 probe-rs）的 Flash 擦除与编程。
- 会话管理、错误分类、结构化输出、MCP 安全注解与 server instructions。
- npm 自动安装入口、原生二进制、三平台 CI、GitFlow、GitHub Pages、Release。

非目标（v1）：

- 不内置任何厂商芯片支持、SVD、FLM 或芯片名称。
- 不做 GUI；只做 MCP stdio 服务器。
- 不做 SWO/SWV 或 trace。
- 不做 RISC-V、Xtensa 等非 Cortex-M 架构。
- 不实现 Streamable HTTP 网络传输（保持本地 stdio）。
- 不自行重写 CMSIS-DAP / SWD / ADIv5 协议栈。

## 3. 关键技术决策

| 编号 | 决策 | 理由 |
| --- | --- | --- |
| D1 | Rust 独立程序 + probe-rs 库 + 官方 Rust MCP SDK（rmcp） | 用户零依赖；probe-rs 提供成熟协议栈；rmcp 提供标准 MCP 实现 |
| D2 | 仅 stdio 传输；npm 自动安装 + GitHub Release 原生程序双入口 | 贴近用户“标准 MCP 配置”的预期，同时保留零运行时安装路径 |
| D3 | 默认使用通用 Cortex-M 目标；SVD 与 Flash 算法为运行时可选资源 | 任意标准 Cortex-M 可基础调试；不把芯片资料打进仓库 |
| D4 | 三级安全策略（只读 / 写 / 破坏性） | 平衡易用性与防误操作 |
| D5 | 仓库不包含、不引用任何厂商专有资料 | 公开仓库保持纯通用 CMSIS-DAP MCP 性质 |
| D6 | GitFlow + 三平台 CI + GitHub Pages + Release | 用户明确要求的开发与发布流程 |

## 4. 系统架构

```text
MCP 客户端（Codex / Claude / 任意 AI 主机）
        |  stdio JSON-RPC 2.0
        v
cmsis-dap-mcp（Rust 单进程）
  |-- MCP 工具层：probe / memory / core / svd / flash / dap
  |-- 安全策略层：read-only / write / destructive
  |-- 会话管理：探针选择、连接状态、断点状态
  |-- probe-rs 适配层：封装 probe-rs API 为内部后端接口
        |  USB HID/Bulk
        v
CMSIS-DAP 探针 -> SWD/JTAG -> Cortex-M 目标芯片

运行时可选资源（用户提供路径，不进入仓库）：
  SVD 文件（命名外设访问）
  CMSIS-Pack / FLM / target YAML（Flash 算法）
```

一次工具调用的数据流：

1. MCP 客户端调用工具（JSON-RPC over stdio）。
2. 安全策略校验工具等级与当前开关（如破坏性操作未开启则拒绝）。
3. 会话管理选择或建立连接（探针、协议、速度）。
4. probe-rs 适配层执行 SWD/JTAG 事务。
5. 结果转换为结构化输出与 MCP 文本内容返回；错误分类并置 `is_error`。

## 5. 模块职责

### 5.1 cli

- 解析启动参数：`--allow-destructive`、`--log-level`、`--log-file`、`--probe-id`、`--protocol`、`--speed-khz`。
- 初始化日志（stderr / 日志文件），启动 MCP stdio 服务。

### 5.2 mcp

- 基于 rmcp 注册工具、资源与 server instructions。
- 每个工具声明 MCP annotations（`readOnlyHint`、`destructiveHint`、`idempotentHint`）。
- server instructions 首 512 字符自包含，说明安全等级、SVD/Flash 用法与常见流程。

### 5.3 security

- 工具等级表：`ReadOnly` / `Write` / `Destructive`。
- 破坏性操作仅在 `--allow-destructive` 时可用；其余情况返回明确的 `DestructiveDisabled` 错误。
- 写操作不隐式放行，交由 MCP 客户端策略（approval mode）决定。

### 5.4 session

- 单活动会话；`connect` 建立、`disconnect` 释放。
- 保存探针选择、协议、速度、当前 SVD、断点集合与 Flash target 描述。
- 重复 `connect` 前自动断开旧会话（在返回信息中说明）。

### 5.5 backend（probe-rs 适配层）

- 封装 probe-rs 的探针枚举、attach、内存接口、核心接口、DAP 原始访问与 Flash 下载。
- 将 probe-rs 错误映射为 MCP 错误分类。
- 该层是唯一直接依赖 probe-rs 的模块，便于将来替换后端或增加模拟后端。

### 5.6 svd

- 通过用户路径加载 SVD，解析外设/寄存器/位域（实施时优先评估 `svd-parser` crate，否则用最小解析器）。
- 解析结果仅存内存，不写入仓库。
- 命名访问 = 外设基址 + 寄存器偏移（+ 位域掩码）。

### 5.7 flash

- 需要用户提供 CMSIS-Pack（`.pack`/`.pdsc`）或 target YAML；用 probe-rs 加载 Flash 算法。
- 所有擦除/编程工具均为 Destructive 等级。

## 6. MCP 工具清单

| 工具 | 用途 | 等级 | 注解 |
| --- | --- | --- | --- |
| `list_probes` | 枚举 CMSIS-DAP 探针 | ReadOnly | readOnly |
| `get_probe_info` | 探针固件/序列号/能力 | ReadOnly | readOnly |
| `connect` | 连接目标（协议/速度/目标名） | Write | - |
| `disconnect` | 断开连接 | Write | idempotent |
| `get_target_info` | DP/AP、CoreSight、核心信息 | ReadOnly | readOnly |
| `read_memory` | 按地址/宽度读取内存 | ReadOnly | readOnly |
| `write_memory` | 按地址/宽度写入内存 | Write | - |
| `read_core_register` | 读取核心寄存器 | ReadOnly | readOnly |
| `write_core_register` | 写入核心寄存器 | Write | - |
| `halt` | 暂停核心 | Write | - |
| `resume` | 恢复运行 | Write | - |
| `step` | 单步 | Write | - |
| `set_breakpoint` | 设置硬件断点 | Write | - |
| `clear_breakpoints` | 清除断点 | Write | idempotent |
| `list_breakpoints` | 列出断点 | ReadOnly | readOnly |
| `reset` | 复位目标 | Write | destructiveHint 提示 |
| `read_dap` | 原始 DP/AP 读取 | ReadOnly | readOnly |
| `write_dap` | 原始 DP/AP 写入 | Write | - |
| `load_svd` | 加载用户 SVD | Write | - |
| `list_peripherals` | 列出外设 | ReadOnly | readOnly |
| `read_peripheral` | 命名读取寄存器/位域 | ReadOnly | readOnly |
| `write_peripheral` | 命名写入寄存器/位域 | Write | - |
| `erase_flash` | 擦除 Flash | Destructive | destructive |
| `program_flash` | 烧写二进制 | Destructive | destructive |

## 7. 会话模型

- 单活动会话，进程生命周期内可多次连接/断开。
- 连接参数：探针 ID（可空，自动选择唯一探针）、协议（`swd` 默认 / `jtag`）、时钟 kHz、目标名（可空，默认通用 `cortex_m`）。
- 目标名用于加载用户 target 描述（Flash 场景）；普通调试不需要。
- 工具超时由 MCP 客户端 `tool_timeout_sec` 控制；长事务（Flash）返回进度文本。

## 8. 输出与错误

- 成功：`structuredContent` + 文本摘要；失败：`is_error=true` + 分类错误码。
- 错误分类：`ProbeNotFound`、`ConnectFailed`、`NotConnected`、`ProtocolError`、`Timeout`、`MemoryFault`、`SvdNotLoaded`、`UnsupportedFeature`、`DestructiveDisabled`、`InvalidArgument`、`InternalError`。
- 日志只写 stderr 或日志文件，绝不写 stdout（防止污染 MCP 协议）。

## 9. 安全设计

- 只读工具默认放行。
- 写/调试控制工具标记为写，由 MCP 客户端 approval 策略决定。
- 破坏性工具默认不可调用，仅 `--allow-destructive` 后可用；错误信息明确说明如何开启。
- README 与 server instructions 明确风险：Flash 擦写、Option Bytes、读保护与解锁可能导致设备失效。

## 10. CLI 与配置

```text
cmsis-dap-mcp [--allow-destructive] [--log-level debug|info|warn|error]
              [--log-file <path>] [--probe-id <id>] [--protocol swd|jtag]
              [--speed-khz <khz>] [--target <name>] [--svd <path>]
```

环境变量可覆盖：`CMSIS_DAP_MCP_ALLOW_DESTRUCTIVE`、`CMSIS_DAP_MCP_SVD`、`CMSIS_DAP_MCP_TARGET` 等（实施时固定清单）。

## 11. 测试策略

- 单元测试：SVD 解析、地址解析、安全策略、错误映射、CLI 参数。
- 后端测试：使用 probe-rs 的模拟/伪探针验证连接、内存读写、核心控制流程。
- CI：`cargo fmt --check`、`cargo clippy`、`cargo test`；无硬件依赖。
- 本地实机验证（不入 CI、不入库）：ALB32F033C8 + 现有 CMSIS-DAP v1 探针（VID 0416 / PID 5051），使用 SDK 的 SVD 与 FLM 生成本地 target 描述，验证读 RAM、写 RAM、核心暂停/恢复、SVD 命名访问；破坏性验证在用户确认下进行。
- 发布前验证清单：三平台构建产物、npm 包 dry-run、Pages 构建、GitFlow 合并结果、`git diff --check`、仓库无厂商内容扫描。

## 12. 打包与发布

- 原生产物：Windows `cmsis-dap-mcp.exe`、Linux/macOS `cmsis-dap-mcp`，按平台压缩上传 Release。
- npm：平台专属 `optionalDependencies` 包（如 `cmsis-dap-mcp-win32-x64`）+ 元包 `cmsis-dap-mcp`；`npx -y cmsis-dap-mcp` 即可启动（发布前确认包名/scope）。
- Linux 用户需一次 udev 规则授权 USB（操作系统权限要求，随文档提供）。
- 版本化：SemVer；tag `vX.Y.Z` 触发 Release 工作流。

## 13. CI/CD 与 GitFlow

- GitHub Actions：`ci.yml`（三平台 fmt/clippy/test/build）、`release.yml`（tag 触发构建产物 + npm publish + Pages 部署）、`pages.yml`（文档构建）。
- GitFlow 本地流程：`main` -> `develop`；功能分支 `feature/<topic>` -> `develop`；发布分支 `release/vX.Y.Z` -> `main`。
- 全部功能本地验证完成后才推送远程并发布。

## 14. 文档与 GitHub Pages

- `docs/` 使用 mdBook，内容：快速开始、MCP 配置示例（npx / 原生）、工具参考、安全说明、SVD/Flash 用法、探针兼容性。
- GitHub Pages 地址在仓库创建后确定。

## 15. 仓库结构与许可证

```text
Cargo.toml / Cargo.lock
src/            （cli、mcp、security、session、backend、svd、flash）
tests/          （集成/回归测试）
npm/            （发布元包与平台包描述）
docs/           （mdBook 文档与设计规格）
.github/workflows/
README.md
LICENSE-APACHE / LICENSE-MIT
```

许可证：Apache-2.0 与 MIT 双许可（与 probe-rs 生态一致）；第三方依赖清单在发布前核对。

## 16. 里程碑

| 里程碑 | 内容 | 验收 |
| --- | --- | --- |
| M0 | 安装 Rust 工具链、初始化仓库、Cargo 骨架、MCP hello | `cargo test` 通过 |
| M1 | 探针枚举 + connect + 内存读写 | 模拟后端测试通过；实机读 RAM |
| M2 | 核心控制：halt/resume/step/断点/寄存器/复位 | 实机验证通过 |
| M3 | SVD 加载与命名外设访问 | ALB32F0xx.svd 本地解析与命名读取 |
| M4 | Flash 擦写（destructive） | 本地目标描述验证（用户确认后） |
| M5 | npm 包装 + 三平台 CI + Pages 文档 | 三平台产物与 npm dry-run 通过 |
| M6 | 本地全量验证 + GitFlow 合并 | `feature -> develop -> main` |
| M7 | 推送远程、Release 发布、公开文档 | Release 可下载，npx 配置可用 |

## 17. 开放项（发布前需用户提供）

- GitHub 仓库 owner/名称与推送凭据（`gh` 未安装，届时使用 HTTPS 凭据或安装 `gh`）。
- npm 包名/scope 与发布凭据。
- 实机验证时的破坏性操作确认（Flash 擦写实验）。

## 18. 本地验证环境（仅本机，不入库）

- 芯片：ALB32F033C8（Cortex-M0）。
- 探针：当前 CMSIS-DAP v1 复合设备，VID 0416 / PID 5051，固件 1.2.0，序列号 CDAB1A795BBD42E239E339E3。
- 资料：`C:\Workspace\ALB32MCU\workspace\ALB32F0xx_SDK`（SVD、DFP、FLM），仅用于本地验证与生成 target 描述。
- 验证方式：从 SDK 资料生成 probe-rs target 描述置于仓库外目录；所有验证命令与结果记录在本地，不提交。