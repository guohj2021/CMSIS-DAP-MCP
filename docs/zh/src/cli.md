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
`--target`、`--under-reset`、`--target-yaml`、`--svd`、`--yes`、`--json`、
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
cmsis-dap-cli --yes flash program --address 0x08000000 --file fw.hex --verify
cmsis-dap-cli script --file flash.jlink
cmsis-dap-cli repl
```

REPL 中输入 `?`/`help` 查看支持的 J-Link / OpenOCD 风格命令，`q`/`exit` 退出。

## 输出与退出码

- 默认输出人类可读；加 `--json` 输出与 MCP 工具一致的机器可读 JSON。日志
  始终写 stderr。
- 退出码：`0` 成功，`1` 运行时错误，`2` 用法错误，`3` 确认被拒或破坏性
  操作缺少确认。

## 破坏性操作

`flash erase`、`flash program` 以及脚本中的破坏性命令（`erase`、`loadbin`、
`loadfile`、`flash write_image`、`flash erase_sector`）受门禁保护：

- 有终端时要求交互确认（除非带 `--yes`）。
- 无终端时必须带 `--yes`，否则命令被拒绝并以退出码 3 结束。
- REPL 默认只读；可在提示时交互开启破坏性模式，或用 `--yes` 启动。

Flash 擦除与烧录可能导致设备永久损坏。确认前请仔细核对地址与文件。
