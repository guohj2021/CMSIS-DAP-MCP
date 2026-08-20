 # 从零开始

 本指南面向零基础用户，手把手带你从安装软件到连接硬件、读写内存，
 逐步掌握 CMSIS-DAP MCP 工具的全部能力。每一步都附有具体命令和预期输出。

 ## 你将获得什么

 CMSIS-DAP MCP 提供两个共用同一引擎的工具：

 - **cmsis-dap-mcp** —— MCP（模型上下文协议）服务器，让 AI 助手（Codex、
   Claude Code 等）直接操控你的调试探针和目标芯片。
 - **cmsis-dap-cli** —— 独立命令行工具，无需 AI 客户端，直接在终端中
   调试、烧录和监控目标。

 两者都支持：

 - 枚举调试探针、通过 SWD 或 JTAG 连接 Cortex-M 芯片
 - 读写内存与内核寄存器、暂停/恢复/单步执行
 - 运行时加载 SVD 文件，按名称访问外设寄存器
 - 从固件文件（elf/axf/bin/hex）烧录 Flash
 - 运行 J-Link / OpenOCD 风格调试脚本
 - 非侵入式 CPU 快照（不复位目标）
 - TCP 远程服务器与 GDB 调试服务器

 CLI 额外提供实时调试：`watch`（变量轮询）、`rtt monitor`（SEGGER RTT 日志）、
 `evr monitor`（CMSIS-View Event Recorder）—— 全部走 SWD/JTAG，无需串口。

 ---

 ## 需要什么硬件

 ### 必需

 1. **CMSIS-DAP 调试探针**
    - 支持 CMSIS-DAP v1（HID）或 v2（WinUSB）协议
    - 大多数市售 CMSIS-DAP 兼容探针均可使用
    - 通过 USB 连接电脑

 2. **Cortex-M 开发板**
    - 任何带 SWD 调试端口的 ARM Cortex-M 开发板
    - 支持 M0、M0+、M3、M4、M7 全系列内核
    - 探针和开发板之间通过 SWD 线连接

 3. **SWD 连接线**
    - 至少 3 根线：**SWDIO**、**SWCLK**、**GND**
    - 将探针的 SWD 引脚连接到开发板对应的调试端口

 ### 可选

 4. **nRST 复位线**
    - 用于 `under_reset` 模式连接（锁定或无响应的目标）
    - 连接探针的 nRST 引脚到开发板的复位引脚

 ### 接线示意

 ```text
 探针 (CMSIS-DAP)          开发板 (Cortex-M)
 ┌─────────────┐           ┌─────────────┐
 │  SWDIO  ──────┼───────────┤  SWDIO      │
 │  SWCLK  ──────┼───────────┤  SWCLK      │
 │  GND    ──────┼───────────┤  GND        │
 │  nRST   ──────┼── (可选) ─┤  NRST       │
 └──────┬──────┘           └─────────────┘
        │ USB
     ┌──┴──┐
     │ PC  │
     └─────┘
 ```

 > **提示**：不同的探针和开发板引脚定义不同，请参照你硬件的引脚图确认
 > SWDIO/SWCLK/GND 的对应位置。

 ---

 ## 需要什么文件

 工具按功能层级递进，不同功能需要不同的文件：

 | 功能 | 需要的文件 | 来源 |
 | --- | --- | --- |
 | 基础调试（内存、寄存器、执行控制） | 无，开箱即用 | — |
 | 命名外设访问 | SVD 文件 | 芯片厂商 SDK 或 CMSIS-Pack |
 | Flash 烧录 | Keil FLM 闪存算法文件 | IDE 安装目录或芯片厂商 |
 | 符号级调试（watch/RTT/EVR） | 固件 ELF 或 AXF 文件 | 你的编译输出 |

 **FLM 文件**通常位于 Keil MDK 的 `Flash/` 目录下，文件名形如
 `TargetChip_64.FLM`。

 **SVD 文件**描述芯片外设寄存器布局，通常随芯片 SDK 或 CMSIS-Pack 提供，
 文件名形如 `TargetChip.svd`。

 > **注意**：本仓库不捆绑任何芯片专有数据。所有文件由用户在运行时提供。

 ---

 ## 环境安装

 ### 方式一：使用 npm（推荐）

 npm 是 Node.js 的包管理器，本项目的两个工具都已发布为 npm 包。

 #### Windows

 ```bash
 # 用 winget 安装 Node.js（包含 npm）
 winget install OpenJS.NodeJS.LTS

 # 或用 scoop
 scoop install nodejs-lts
 ```

 安装完成后打开新的终端窗口，验证：

 ```bash
 node --version    # 应显示 v18.x 或更高
 npm --version     # 应显示 9.x 或更高
 ```

 #### Linux（Debian/Ubuntu）

 ```bash
 curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
 sudo apt install -y nodejs
 ```

 #### Linux（Fedora/RHEL）

 ```bash
 sudo dnf install -y nodejs npm
 ```

 #### macOS

 ```bash
 brew install node
 ```

 ### 方式二：使用原生二进制（离线场景）

 如果无法安装 Node.js，可以从 GitHub Releases 下载对应平台的原生二进制，
 无需任何运行时依赖。

 ### 方式三：从源码构建（开发者）

 需要安装 Rust 工具链：

 ```bash
 curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
 cargo build --release --workspace
 ```

 ---

 ## 安装工具

 ### MCP 服务器（给 AI 助手用）

 ```bash
 # 验证可以运行（零安装，首次运行会自动下载）
 npx -y cmsis-dap-mcp --help
 ```

 ### 命令行工具（给人用）

 ```bash
 # 零安装快速试用
 npx -y cmsis-dap-cli --help

 # 或全局安装后直接使用命令
 npm install -g cmsis-dap-cli
 cmsis-dap-cli --help
 ```

 如果你从 GitHub Releases 下载了原生二进制，把它放在 PATH 中或直接用
 完整路径调用。

 ---

 ## 驱动安装

 ### Windows

 - **CMSIS-DAP v1（HID）**：通常免驱，插入即可识别。
 - **CMSIS-DAP v2（WinUSB）**：需要安装 WinUSB 驱动。
   1. 从 <https://zadig.akeo.ie/> 下载 Zadig
   2. 插入探针，打开 Zadig
   3. 在菜单中选择 `Options → List All Devices`
   4. 选择你的 CMSIS-DAP 设备
   5. 将驱动替换为 **WinUSB**，点击 `Replace Driver`

 ### Linux

 需要添加 udev 规则以允许非 root 用户访问 USB 设备：

 ```bash
 # 创建规则文件（将 xxxx/yyyy 替换为你的探针 VID/PID）
 echo 'SUBSYSTEM=="usb", ATTRS{idVendor}=="xxxx", ATTRS{idProduct}=="yyyy", MODE="0666"' \
   | sudo tee /etc/udev/rules.d/99-cmsis-dap.rules

 # 重新加载规则
 sudo udevadm control --reload-rules
 sudo udevadm trigger

 # 重新插拔探针
 ```

 > **提示**：在 Windows 上查看设备管理器中探针的 VID/PID，或在 Linux 上
 > 用 `lsusb` 查看。

 ### macOS

 通常开箱即用。如果探针无法识别，检查系统设置 > 隐私与安全性中是否有
 USB 设备权限提示。

 ---

 ## 第一步：连接你的硬件

 ### 1. 确认识别

 插入 CMSIS-DAP 探针，打开终端：

 ```bash
 cmsis-dap-cli list
 ```

 预期输出（探针 id 和产品名会因硬件不同而不同）：

 ```text
 CMSIS-DAP probes found:
   id        : 0123456789AB
   product   : CMSIS-DAP
   serial    : (none)
   protocols : SWD, JTAG
 ```

 如果列表为空，请检查[驱动安装](#驱动安装)和 USB 连接。

 ### 2. 连接目标芯片

 ```bash
 cmsis-dap-cli connect
 ```

 这会自动探测目标芯片。如果你知道芯片型号，可以指定以获得更详细的
 内存映射信息：

 ```bash
 cmsis-dap-cli --target STM32F030C8 connect
 ```

 预期输出：

 ```text
 target: {"ap_count":1, "core_count":1, "core_type":"Armv6m", ...,
          "memory_regions":[FLASH 0x08000000-0x08010000, SRAM 0x20000000-0x20002000]}
 ```

 ### 3. 读内存验证连接

 ```bash
 cmsis-dap-cli read --address 0x20000000 --width u32 --count 4
 ```

 预期输出（值取决于目标芯片当前的内存内容）：

 ```text
 address: 0x20000000, width: u32, count: 4
   0x20000000: 0x00000040
   0x20000004: 0x00000001
   0x20000008: 0x00000003
   0x2000000C: 0x00000000
 ```

 ### 4. 暂停、读寄存器、恢复

 ```bash
 cmsis-dap-cli halt
 cmsis-dap-cli reg get pc
 cmsis-dap-cli resume
 ```

 预期输出：

 ```text
 halted: true
 pc = 0x0800122A
 running: true
 ```

 **恭喜！** 你已经成功连接到目标芯片并完成了基本的内存和寄存器操作。

 > **提示**：也可以用 `repl` 进入交互模式，保持一个会话持续操作：
 >
 > ```bash
 > cmsis-dap-cli repl
 > # 在提示符下依次输入 connect → halt → reg pc → resume
 > ```

 ---

 ## 进阶：MCP 服务器配置

 如果你使用 AI 助手（如 Codex、Claude Code 或 opencode），可以让它直接
 操控探针。只需添加 MCP 服务器配置：

 ### Codex

 ```bash
 codex mcp add cmsis-dap -- npx -y cmsis-dap-mcp
 ```

 ### Claude Code

 ```bash
 claude mcp add --scope local cmsis-dap -- npx -y cmsis-dap-mcp
 ```

 ### opencode

 ```bash
 opencode mcp add cmsis-dap -- npx -y cmsis-dap-mcp
 ```

 添加后重启客户端。你可以在 AI 对话中直接说：

 > 列出已连接的调试探针，然后连接到目标芯片。

 AI 会自动调用 `list_probes`、`connect` 等工具完成操作。

 ---

 ## 进阶：Flash 烧录

 ### 准备工作

 1. 确认你的芯片有对应的 FLM 闪存算法文件
 2. 知道芯片的 Flash 和 SRAM 地址范围（查看芯片数据手册）

 ### CLI 方式

 ```bash
 # 第一步：从 FLM 生成 target YAML（只需做一次）
 cmsis-dap-cli chip generate \
   --flm /path/to/TargetChip.FLM \
   --flash-start 0x08000000 --flash-size 0x10000 \
   --sram-start 0x20000000 --sram-size 0x2000 \
   --name TargetChip --output TargetChip.yaml

 # 第二步：使用生成的 YAML 连接并烧录
 cmsis-dap-cli --target-yaml TargetChip.yaml connect
 cmsis-dap-cli flash erase --address 0x08000000 --size 0x10000
 cmsis-dap-cli flash program --address 0x08000000 --file firmware.hex --verify
 ```

 ### MCP 方式

 ```text
 define_chip {
   "flm": "/path/to/TargetChip.FLM",
   "flash_start": 0x08000000, "flash_size": 0x10000,
   "sram_start": 0x20000000, "sram_size": 0x2000,
   "core": "armv6m", "name": "TargetChip"
 }
 connect { "target": "TargetChip", "protocol": "swd" }
 program_flash { "address": 0x08000000, "path": "firmware.hex", "format": "hex", "verify": true }
 ```

 ### 开启破坏性模式

 Flash 擦除和烧录是破坏性操作，默认禁用。有两种开启方式：

 - **启动时**：加 `--allow-destructive` 参数
 - **运行时**：调用 `update_config {"allow_destructive": true}`（无需重启）

 ---

 ## 进阶：命名外设（SVD）

 SVD 文件描述芯片的外设寄存器布局，让你用名称而非地址操作外设。

 ```bash
 # CLI 方式
 cmsis-dap-cli --svd TargetChip.svd svd list
 cmsis-dap-cli --svd TargetChip.svd svd read GPIOA.ODR.ODR0
 cmsis-dap-cli --svd TargetChip.svd svd write GPIOA.ODR.ODR0 1
 ```

 ```text
 # MCP 方式
 load_svd { "path": "/path/to/TargetChip.svd" }
 list_peripherals {}
 read_peripheral { "peripheral": "GPIOA", "register": "ODR", "field": "ODR0" }
 write_peripheral { "peripheral": "GPIOA", "register": "ODR", "field": "ODR0", "value": 1 }
 ```

 ---

 ## 进阶：实时调试

 CLI 独有三项实时调试能力，全部走 SWD/JTAG——无需串口：

 ### 变量轮询（watch）

 ```bash
 cmsis-dap-cli --elf firmware.axf watch counter --interval-ms 200 --count 0
 ```

 ### RTT 日志

 ```bash
 cmsis-dap-cli --elf firmware.axf rtt monitor --channel 0 --count 0
 ```

 ### Event Recorder

 ```bash
 cmsis-dap-cli --elf firmware.axf evr monitor --count 0
 ```

 > **注意**：实时调试需要固件 ELF 文件（`--elf`），且目标固件需要已初始化
 > 对应的组件（SEGGER RTT 或 CMSIS-View Event Recorder）。

 ---

 ## 下一步

 - [快速开始](./quickstart.md) —— MCP 服务器快速上手
 - [AI 客户端配置](./ai-clients.md) —— 各 AI 客户端的详细配置
 - [工具参考](./tools.md) —— MCP 工具完整参考
 - [命令行工具](./cli.md) —— CLI 命令完整参考
 - [脚本使用](./scripting.md) —— J-Link / OpenOCD 风格脚本
 - [SWD 与 JTAG](./swd-jtag.md) —— 协议选择指南
 - [SVD 与 Flash](./svd-flash.md) —— 外设访问与烧录工作流
 - [故障排查](./troubleshooting.md) —— 常见问题与解决方案
