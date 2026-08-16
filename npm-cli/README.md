# cmsis-dap-cli

[English](#english) · [中文](#chinese)

---

<a id="english"></a>
# English

A standalone command-line tool for working with CMSIS-DAP debug probes and
Cortex-M chips over **SWD** or **JTAG**. It shares the same engine as the
`cmsis-dap-mcp` server, so you can do everything from a terminal or a script
without an AI client: enumerate probes, read/write memory and core registers,
control execution, use named peripherals via SVD files, program flash, run
J-Link / OpenOCD style scripts, and enter an interactive shell.

## Install

```bash
npx -y cmsis-dap-cli --help
```

The npm package downloads the correct platform binary automatically
(`cmsis-dap-cli-win32-x64`, `cmsis-dap-cli-linux-x64`,
`cmsis-dap-cli-darwin-x64`). Native binaries are also published on
[GitHub Releases](https://github.com/guohj2021/CMSIS-DAP-MCP/releases).

## Quick start

```bash
cmsis-dap-cli chip search STM32F030          # find a built-in chip
cmsis-dap-cli --target STM32F030C8 connect   # connect (or --target-yaml for custom chips)
cmsis-dap-cli --target STM32F030C8 read --address 0x20000000 --width u32 --count 4
cmsis-dap-cli --target STM32F030C8 halt
cmsis-dap-cli --target STM32F030C8 reg get pc
cmsis-dap-cli --target STM32F030C8 flash program --address 0x08000000 --file fw.hex --verify
cmsis-dap-cli --target STM32F030C8 repl
```

Commands that need a target auto-connect using the global connection options
(`--probe-id`, `--protocol`, `--speed-khz`, `--target`, `--under-reset`,
`--target-yaml`).

## Command overview

| Command | Purpose |
| --- | --- |
| `list` / `info` | enumerate probes / show probe info |
| `connect` / `disconnect` / `target` | session management |
| `read` / `write` / `verify` | memory access and verification |
| `regs` / `reg get` / `reg set` | core registers |
| `status` / `halt` / `resume` / `step` / `reset` | execution control |
| `bp set/list/clear` / `wp set/list/clear` | breakpoints and watchpoints |
| `dap read/write` | raw DAP (DP/AP) access |
| `svd list/read/write` | named peripheral access (needs `--svd FILE`) |
| `flash erase/program` | flash erase and programming |
| `script --file/--text` | J-Link / OpenOCD style scripts |
| `chip generate/list/search` | generate target YAML from an FLM, list/search chips |
| `repl` | interactive shell |

Use `--json` for machine-readable output. Flash erase/program run directly;
they require a target that defines flash.

## Examples

```bash
cmsis-dap-cli list
cmsis-dap-cli --target STM32F030C8 connect
cmsis-dap-cli --target STM32F030C8 read --address 0x20000000 --width u32 --count 4
cmsis-dap-cli --target STM32F030C8 flash erase --address 0x08000000 --size 0x10000
cmsis-dap-cli --target STM32F030C8 flash program --address 0x08000000 --file fw.hex --verify
cmsis-dap-cli --svd target.svd svd read GPIOA.ODR.ODR0
cmsis-dap-cli --target STM32F030C8 script --file flash.jlink
cmsis-dap-cli --target STM32F030C8 repl
```

For chips not built into probe-rs, generate a target YAML once from the vendor
flash algorithm (FLM), then connect with it:

```bash
cmsis-dap-cli chip generate --flm MyChip_64.FLM \
  --flash-start 0x08000000 --flash-size 0x10000 \
  --sram-start 0x20000000 --sram-size 0x2000 \
  --name MYCHIP --output MYCHIP.yaml
cmsis-dap-cli --target-yaml MYCHIP.yaml connect
```

## Documentation

Full documentation (English and Chinese) is published on GitHub Pages:

- English: <https://guohj2021.github.io/CMSIS-DAP-MCP/cli.html>
- 中文: <https://guohj2021.github.io/CMSIS-DAP-MCP/zh/cli.html>

## License

MIT OR Apache-2.0

---

<a id="chinese"></a>
# 中文

面向 CMSIS-DAP 调试探针与 Cortex-M 芯片的独立命令行工具，通过 **SWD** 或
**JTAG** 工作。它与 `cmsis-dap-mcp` 服务器共用同一套引擎，在终端或脚本里
无需 AI 客户端即可完成：枚举探针、读写内存与内核寄存器、控制执行、用 SVD
访问命名外设、烧录 Flash、运行 J-Link / OpenOCD 风格脚本，以及进入交互式
shell。

## 安装

```bash
npx -y cmsis-dap-cli --help
```

npm 包会自动下载对应平台的二进制（`cmsis-dap-cli-win32-x64`、
`cmsis-dap-cli-linux-x64`、`cmsis-dap-cli-darwin-x64`）。原生二进制同时发布在
[GitHub Releases](https://github.com/guohj2021/CMSIS-DAP-MCP/releases)。

## 快速上手

```bash
cmsis-dap-cli chip search STM32F030          # 查找内置芯片
cmsis-dap-cli --target STM32F030C8 connect   # 连接（自定义芯片用 --target-yaml）
cmsis-dap-cli --target STM32F030C8 read --address 0x20000000 --width u32 --count 4
cmsis-dap-cli --target STM32F030C8 halt
cmsis-dap-cli --target STM32F030C8 reg get pc
cmsis-dap-cli --target STM32F030C8 flash program --address 0x08000000 --file fw.hex --verify
cmsis-dap-cli --target STM32F030C8 repl
```

需要目标的命令会自动使用全局连接参数（`--probe-id`、`--protocol`、
`--speed-khz`、`--target`、`--under-reset`、`--target-yaml`）。

## 命令一览

| 命令 | 用途 |
| --- | --- |
| `list` / `info` | 枚举探针 / 查看探针信息 |
| `connect` / `disconnect` / `target` | 会话管理 |
| `read` / `write` / `verify` | 内存访问与校验 |
| `regs` / `reg get` / `reg set` | 内核寄存器 |
| `status` / `halt` / `resume` / `step` / `reset` | 执行控制 |
| `bp set/list/clear` / `wp set/list/clear` | 断点与数据观察点 |
| `dap read/write` | 原始 DAP（DP/AP）访问 |
| `svd list/read/write` | 命名外设访问（需 `--svd FILE`） |
| `flash erase/program` | Flash 擦除与烧录 |
| `script --file/--text` | J-Link / OpenOCD 风格脚本 |
| `chip generate/list/search` | 从 FLM 生成 target YAML、列出/搜索芯片 |
| `repl` | 交互式 shell |

`--json` 输出机器可读结果。Flash 擦除/烧录直接执行；目标必须定义了 Flash。

## 示例

```bash
cmsis-dap-cli list
cmsis-dap-cli --target STM32F030C8 connect
cmsis-dap-cli --target STM32F030C8 read --address 0x20000000 --width u32 --count 4
cmsis-dap-cli --target STM32F030C8 flash erase --address 0x08000000 --size 0x10000
cmsis-dap-cli --target STM32F030C8 flash program --address 0x08000000 --file fw.hex --verify
cmsis-dap-cli --svd target.svd svd read GPIOA.ODR.ODR0
cmsis-dap-cli --target STM32F030C8 script --file flash.jlink
cmsis-dap-cli --target STM32F030C8 repl
```

probe-rs 内置库没有的芯片，先用厂商 Flash 算法（FLM）生成一次 target YAML，
再连接：

```bash
cmsis-dap-cli chip generate --flm MyChip_64.FLM \
  --flash-start 0x08000000 --flash-size 0x10000 \
  --sram-start 0x20000000 --sram-size 0x2000 \
  --name MYCHIP --output MYCHIP.yaml
cmsis-dap-cli --target-yaml MYCHIP.yaml connect
```

## 文档

完整文档（英文与中文）发布在 GitHub Pages：

- 英文：<https://guohj2021.github.io/CMSIS-DAP-MCP/cli.html>
- 中文：<https://guohj2021.github.io/CMSIS-DAP-MCP/zh/cli.html>

## 许可证

MIT OR Apache-2.0
