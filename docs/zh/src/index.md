# CMSIS-DAP MCP

一个 MCP（模型上下文协议）服务器，让 AI 助手可以直接操作 CMSIS-DAP 调试探针，
通过 **SWD** 或 **JTAG** 访问 Cortex-M 芯片资源。

- 通用 Cortex-M 支持：标准内核无需芯片适配即可调试。
- 命名外设访问：运行时加载任意 CMSIS-SVD 文件；仓库不捆绑任何芯片文件。
- Flash 编程：需要带 CMSIS-Pack 烧写算法的目标描述。
- 终端用户零运行时依赖：单个原生二进制，或通过 npm 安装。
- 跨平台：Windows / Linux / macOS。

本站涵盖安装、AI 客户端配置、完整工具参考、SWD/JTAG 选择、SVD/Flash 工作流
与安全说明。

英文文档：<https://guohj2021.github.io/CMSIS-DAP-MCP/>
