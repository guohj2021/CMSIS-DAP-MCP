# 命令行工具

`cmsis-dap-cli` 是面向人与脚本的独立命令行工具。它与 MCP 服务器共用同一套
`cmsis-dap-core` 引擎（探针枚举、内存、内核控制、SVD、Flash 与脚本），但直接
面向终端用户，不经过 MCP。

## 安装

通过 npm 零安装：

```bash
npx -y cmsis-dap-cli --help
```

或从 [GitHub Releases](https://github.com/guohj2021/CMSIS-DAP-MCP/releases)
下载 Windows / Linux / macOS 原生二进制。

## 命令一览

全局参数（子命令之前）：`--probe-id`、`--protocol swd|jtag`、`--speed-khz`、
`--target`、`--under-reset`、`--target-yaml`、`--svd`、`--json`、
`--log-level`、`--log-file`。

| 命令 | 用途 |
| --- | --- |
| `list` | 枚举已连接探针 |
| `info` | 查看探针信息 |
| `connect` / `disconnect` / `target` | 管理会话并查看目标信息 |
| `read --address A --width W --count N [--output FILE --format bin\|hex]` | 读内存或导出范围到文件 |
| `write --address A --width W --values V1,V2,...` | 写内存 |
| `verify --address A --width W --values ...` | 按期望值校验内存 |
| `regs` / `reg get NAME\|NUM` / `reg set NAME\|NUM VALUE` | 内核寄存器访问 |
| `status` / `halt` / `resume` / `step` / `reset [--mode run\|halt]` | 执行控制 |
| `bp set ADDR` / `bp list` / `bp clear` | 硬件断点 |
| `wp set ADDR --access read\|write\|rw` / `wp list` / `wp clear` | 数据观察点 |
| `dap read ADDR` / `dap write ADDR VALUE` | 原始 DAP（DP/AP）访问 |
| `svd list` / `svd read PERIPH.REG[.FIELD]` / `svd write PERIPH.REG[.FIELD] VALUE` | SVD 命名访问（需 `--svd FILE`） |
| `flash erase --address A --size N` / `flash program --address A --file F [--format elf\|axf\|bin\|hex] [--verify]` | Flash 擦除 / 烧录 |
| `script --file F` 或 `--text TEXT` | 运行 J-Link / OpenOCD 风格脚本 |
| `chip generate --flm F --flash-start A --flash-size N --sram-start A --sram-size N [--name NAME] [--output FILE]` | 从 Keil FLM 生成 probe-rs target YAML |
| `chip list` / `chip search KEYWORD` | 列出或搜索芯片变体（内置库 + `--target-yaml` 自定义芯片） |
| `repl` | 交互式 shell（J-Link Commander 风格） |

需要目标的命令会自动使用全局连接参数建立连接。数字支持十进制或十六进制
（`0x...`）。

## 示例

```bash
cmsis-dap-cli list
cmsis-dap-cli connect --protocol swd --speed-khz 1000
cmsis-dap-cli read --address 0x20000000 --width u32 --count 4
cmsis-dap-cli write --address 0x20000000 --width u32 --values 0xDEADBEEF,1
cmsis-dap-cli halt
cmsis-dap-cli reg get pc
cmsis-dap-cli read --address 0x08000000 --width u8 --count 0x1000 --output fw.bin --format bin
cmsis-dap-cli --svd target.svd svd read GPIOA.ODR.ODR0
cmsis-dap-cli flash program --address 0x08000000 --file fw.hex --verify
cmsis-dap-cli script --file flash.jlink
cmsis-dap-cli repl
```

REPL 中输入 `?`/`help` 查看支持的 J-Link / OpenOCD 风格命令，`q`/`exit` 退出。

## 从 FLM 生成 target YAML

对于 probe-rs 内置库没有的芯片，烧录需要一份描述芯片并内嵌厂商 Flash 算法
的 target YAML。不用手写：`chip generate` 读取 Keil FLM 文件（厂商 Flash
算法），你只需要提供 Flash 与 SRAM 地址范围：

```bash
cmsis-dap-cli chip generate \
  --flm MyChip_64.FLM \
  --flash-start 0x08000000 --flash-size 0x10000 \
  --sram-start 0x20000000 --sram-size 0x2000 \
  --name MYCHIP --output MYCHIP.yaml
```

其余信息全部从 FLM 自动提取：算法指令、入口偏移（`Init`/`ProgramPage`/
`EraseSector`/`EraseChip`）、静态数据基址、FlashDevice 描述符（页大小、
擦除值、扇区大小、超时）与设备名。`--name` 默认取 FLM 文件名；用
`--output -` 把 YAML 打印到 stdout。随后用同一工具加载：

```bash
cmsis-dap-cli --target-yaml MYCHIP.yaml --target MYCHIP connect
```

生成的 YAML 会把算法放在 `SRAM 起始 + 0x20`；请确保提供的 SRAM 范围能容纳
算法（放不下时命令会拒绝生成）。

## 查看与搜索芯片

想知道有哪些芯片可用（用于 `--target` 或 REPL 里的 `device`），可以列出或
搜索 probe-rs 内置芯片库：

```bash
cmsis-dap-cli chip list
cmsis-dap-cli chip search STM32F103
cmsis-dap-cli chip search stm32f103c8
```

搜索不区分大小写，按芯片名字符串匹配。加 `--target-yaml FILE` 可以把
`chip generate` 生成的自定义芯片也纳入列表；`--json` 输出完整信息（所属
系列、内核、Flash 与 RAM 范围），方便脚本处理。

## 输出与退出码

- 默认输出人类可读；加 `--json` 输出与 MCP 工具一致的机器可读 JSON。日志
  始终写 stderr。
- 退出码：`0` 成功，`1` 运行时错误，`2` 用法错误，`3` 确认被拒或破坏性
  操作缺少确认。

## Flash 操作

`flash erase` 与 `flash program` 在 CLI 中直接执行（包括在 `script` 与
`repl` 里），没有确认提示，也没有 `--yes` 参数。但目标必须定义了 Flash：
用 `--target-yaml`/`--target` 连接（脚本/REPL 里 `device NAME` 后重新
`connect`），否则操作会明确报错而不是静默无效果。

Flash 擦除与烧录可能导致设备永久损坏。执行前请仔细核对地址与文件。
