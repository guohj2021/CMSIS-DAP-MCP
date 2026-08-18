# CMSIS-DAP MCP

面向 CMSIS-DAP 调试探针与 Cortex-M 芯片的两个工具，共用同一套引擎，通过
**SWD** 或 **JTAG** 工作：

- **cmsis-dap-mcp** —— MCP（模型上下文协议）服务器，让 AI 助手直接操作探针；
- **cmsis-dap-cli** —— 面向人、脚本与自动化的独立命令行工具。

两者都可以枚举探针、通过 SWD/JTAG 连接、读写内存与内核寄存器、控制执行、
用 SVD 访问命名外设、从固件文件烧录 Flash，并运行 J-Link / OpenOCD 风格
调试脚本。
CLI 还额外提供实时调试能力：`watch`（按可配置刷新间隔轮询变量）、`rtt
monitor`（SEGGER RTT 日志）与 `evr monitor`（CMSIS-View Event Recorder）都
走 SWD/JTAG——无需串口——并支持带时间戳的日志导出。

CLI 还提供 v0.5.0 引入的非侵入调试与远程访问端点：`dump` / MCP
`dump_cpu_state`（不复位目标的 CPU 快照）、`--tcp` / `tcp-server`（远程
JSON-RPC over TCP）与 `--gdb-port` / `gdb-server`（GDB Remote Serial
Protocol stub）。

- 通用 Cortex-M 支持：标准内核无需芯片适配即可调试。
- 命名外设访问：运行时加载任意 CMSIS-SVD 文件；仓库不捆绑任何芯片文件。
- Flash 编程：需要带 CMSIS-Pack 烧写算法的目标描述。
- 终端用户零运行时依赖：单个原生二进制，或通过 npm 安装。
- 跨平台：Windows / Linux / macOS。

本站涵盖安装、AI 客户端配置、MCP 工具与 CLI 完整参考、SWD/JTAG 选择、
SVD/Flash 工作流与安全说明。

英文文档：<https://guohj2021.github.io/CMSIS-DAP-MCP/>
