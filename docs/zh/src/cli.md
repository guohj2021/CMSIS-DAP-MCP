# 命令行工具

## 简介

`cmsis-dap-cli` 是面向人、脚本与自动化的独立命令行工具。它与 MCP 服务器共用
同一套 `cmsis-dap-core` 引擎（探针枚举、内存、内核控制、SVD、Flash 与脚本），
但直接面向终端用户，不经过 MCP。

仓库是包含三个 crate 的 Cargo workspace：

- `cmsis-dap-core` —— 共享引擎（后端、会话、SVD、脚本引擎）；
- `cmsis-dap-mcp` —— MCP 服务器二进制；
- `cmsis-dap-cli` —— 本 CLI，只依赖 `cmsis-dap-core`。

## 安装

发布后可用 npm 零安装：

```bash
npx -y cmsis-dap-cli --help
```

在发布前或离线环境下，从
[GitHub Releases](https://github.com/guohj2021/CMSIS-DAP-MCP/releases) 下载
Windows / Linux / macOS 原生二进制，或本地构建：

```bash
cargo build --release --workspace
./target/release/cmsis-dap-cli --help        # Windows 为 target\release\cmsis-dap-cli.exe
```

想直接敲 `cmsis-dap-cli`，把所在目录加入 `PATH` 即可。

## 快速上手

```bash
cmsis-dap-cli list                                   # 枚举探针
cmsis-dap-cli --probe-id 0123456789AB connect        # 连接（自动选择芯片）
cmsis-dap-cli read --address 0x20000000 --width u32 --count 4
cmsis-dap-cli halt
cmsis-dap-cli reg get pc
cmsis-dap-cli resume
```

需要目标的命令会自动使用全局连接参数连接，典型的一次性会话：

```text
$ cmsis-dap-cli --target STM32F030C8 connect
target: {"ap_count":1,"core_count":1,"core_type":"Armv6m",...,
         "memory_regions":[FLASH 0x08000000-0x08010000, SRAM 0x20000000-0x20002000]}
```

## 全局参数

所有参数都是全局的，可以放在子命令前后。

| 参数 | 含义 |
| --- | --- |
| `--probe-id ID` | 多探针时按 id/序列号选择 |
| `--protocol swd\|jtag` | 调试协议（默认 `swd`） |
| `--speed-khz N` | SWD/JTAG 时钟（kHz） |
| `--target NAME` | 目标芯片名（内置库或 `--target-yaml` 中的变体） |
| `--under-reset` | 按住复位连接（锁定/无响应目标） |
| `--target-yaml FILE` | 加载 target YAML（芯片 + Flash 算法定义） |
| `--svd FILE` | SVD 文件（`svd` 子命令用） |
| `--elf FILE` | 固件 ELF（`symbols`/`watch`/`rtt`/`evr` 符号解析用） |
| `--json` | 输出机器可读 JSON 而非人类文本 |
| `--log-level LEVEL` | 日志过滤级别；日志只写 stderr（默认 `warn`） |
| `--log-file FILE` | 日志写入文件而非 stderr |

数字（地址、大小、数值）支持十进制与十六进制（`0x...`）。

## 命令参考

### 探针与会话

| 命令 | 用途 |
| --- | --- |
| `list` | 枚举已连接探针 |
| `info` | 查看探针信息（id、厂商、产品、序列号、能力） |
| `connect` | 连接目标并显示目标信息 |
| `disconnect` | 断开会话 |
| `target` | 显示目标信息（自动连接） |

### 内存

| 命令 | 用途 |
| --- | --- |
| `read --address A --width W --count N [--output FILE --format bin\|hex]` | 读内存；带 `--output` 时导出范围到文件（此时 `count` 为字节数） |
| `write --address A --width W --values V1,V2,...` | 写内存 |
| `verify --address A --width W --values ...` | 按期望值校验内存 |

`width` 为 `u8`、`u16`、`u32` 或 `u64`。

```bash
cmsis-dap-cli read --address 0x20000000 --width u32 --count 4
cmsis-dap-cli read --address 0x08000000 --width u8 --count 0x1000 --output fw.bin --format bin
cmsis-dap-cli write --address 0x20000000 --width u32 --values 0xDEADBEEF,1,2
```

### 内核

| 命令 | 用途 |
| --- | --- |
| `regs` | 列出内核寄存器名 |
| `reg get NAME\|NUM` | 读寄存器（名字或编号） |
| `reg set NAME\|NUM VALUE` | 写寄存器 |
| `status` | 显示内核状态、停机原因与 PC |
| `halt` / `resume` / `step` | 暂停 / 恢复 / 单步 |
| `reset [--mode run\|halt]` | 复位后继续，或复位后暂停 |

核心在运行时读寄存器会失败——先 `halt`（一次性命令每次是新会话，请在
`script`/`repl` 里 halt 后再读）：

```bash
cmsis-dap-cli script --text "connect\nhalt\nreg pc\nresume"
```

### 断点与数据观察点

```text
bp set ADDR | bp list | bp clear
wp set ADDR --access read|write|rw | wp list | wp clear
```

### DAP

```text
dap read ADDR
dap write ADDR VALUE
```

原始 DP/AP 寄存器访问（`ADDR` 的 bit24..31 选择 AP，低位是寄存器）。

### SVD（命名外设访问）

```text
svd list
svd read PERIPH.REG[.FIELD]
svd write PERIPH.REG[.FIELD] VALUE
```

需要 `--svd FILE`。目标写法：`GPIOA.ODR` 或 `GPIOA.ODR.ODR0`；位域写是
读-改-写。

```bash
cmsis-dap-cli --svd target.svd svd list
cmsis-dap-cli --svd target.svd svd read GPIOA.ODR.ODR0
cmsis-dap-cli --svd target.svd svd write GPIOA.ODR.ODR0 1
```

### Flash

```text
flash erase --address A --size N
flash program --address A --file FILE [--format elf|axf|bin|hex] [--verify]
```

擦除/烧录直接执行（无确认）。目标必须定义了 Flash，否则命令明确报错而不是
静默无效果。`--format` 默认按文件扩展名推断；`--verify` 会读回校验。

```bash
cmsis-dap-cli flash erase --address 0x08000000 --size 0x1000
cmsis-dap-cli flash program --address 0x08000000 --file fw.hex --verify
```

### 脚本

```text
script --file FILE
script --text TEXT
```

执行 J-Link Commander / OpenOCD 风格脚本（见[脚本使用](./scripting.md)）。
`script` 命令会继承全局连接参数，脚本里的 `connect` 直接使用它们。

### 芯片工具

```text
chip generate --flm FILE --flash-start A --flash-size N --sram-start A --sram-size N [--name NAME] [--output FILE]
chip list
chip search KEYWORD
```

`chip generate` 从 Keil FLM 生成 probe-rs target YAML（见下文）。`chip list`/
`chip search` 列出或搜索内置芯片库（也可包含 `--target-yaml` 自定义芯片）；
结果带 Flash/RAM 范围，一眼能看出能不能烧录。

### 符号

```text
symbols list [PATTERN]
symbols resolve NAME
```

查看 `--elf` 固件的符号表。`list` 列出全部符号（可按大小写不敏感的子串
过滤）及虚拟地址；`resolve` 查询单个名字。`watch`/`rtt`/`evr` 正是用同一套
符号去定位变量和控制块。

```bash
cmsis-dap-cli --elf firmware.axf symbols resolve counter
cmsis-dap-cli --elf firmware.axf symbols list counter
```

### 变量实时观察（Live watch）

```text
watch [--interval-ms N] [--count N] [--width u8|u16|u32|u64]
      [--log-dir DIR | --log-file FILE] TARGET...
```

按刷新间隔轮询一个或多个变量并打印带时间戳的采样行。`TARGET` 可以是符号名
（经 `--elf` 解析）或 `0xADDR` 地址。默认 `--interval-ms 500`、`--count 1`
（采样一次）、`--width u32`。`--count 0` 一直运行到 Ctrl-C；干净停止后退出码
为 0，并在 stderr 打印 `stopped (Ctrl-C)`。

```bash
cmsis-dap-cli --target STM32F030C8 --elf firmware.axf \
  watch counter 0x20000004 --interval-ms 200 --count 0
```

示例输出（CMSIS-DAP 探针 + Cortex-M0+ 目标实测）：

```text
[2026-08-16 19:16:13.302] watch_var = 0x00001007
[2026-08-16 19:16:13.520] watch_var = 0x0000100E
[2026-08-16 19:16:13.736] watch_var = 0x00001015
```

### RTT（J-Link RTT 日志）

```text
rtt info
rtt monitor --channel 0,1 [--interval-ms N] [--count N]
            [--address A] [--log-dir DIR | --log-file FILE]
```

`rtt info` 附着目标 RTT 控制块并列出上行通道。`rtt monitor` 轮询所选上行
通道（逗号列表，默认 `0`），每收到一段数据就打印带主机时间戳和通道前缀的
一行（`[RTT0 "Channel 0"] ...`）。控制块地址依次取自 `--elf` 的
`_SEGGER_RTT` 符号、`--address`，或扫描目标 RAM（扫描需要芯片目标定义了
RAM：内置芯片或 `--target-yaml`）。默认 `--interval-ms 200`、`--count 0`
（直到 Ctrl-C）、每通道每轮 `--max-bytes 1024`。

固件需要运行 SEGGER RTT（例如 `rtt_target` 或 SEGGER RTT 实现），并且主机
附着前控制块已初始化。

```bash
cmsis-dap-cli --target STM32F030C8 --elf firmware.axf \
  rtt monitor --channel 0 --count 0 --log-dir logs
```

### Event Recorder（CMSIS-View）

```text
evr info
evr monitor [--interval-ms N] [--count N]
            [--ctx 0..7] [--address A]
            [--log-dir DIR | --log-file FILE]
```

`evr info` 附着片上 Event Recorder 并报告协议版本、记录数、时间戳频率与
计数器。`evr monitor` 通过纯 SWD/JTAG 内存读（无需 trace 硬件、无需串口）
轮询环形缓冲，并按官方 16 字节记录布局解码每个新事件：主机时间戳、目标侧
tick 数与秒数（按 `ts_freq` 换算）、事件上下文（记录 `info` 的 bit16..18，
取值 0..7）、组件与消息编号、序号以及
两个 32 位数值。`--ctx` 可按上下文过滤（可重复或逗号列表）。注意片上记录
只存 16 位事件 id（组件 + 消息）；API 层的 level 用于固件内过滤，不写入记录。

固件需要包含 CMSIS-View Event Recorder 组件（符号 `EventRecorderInfo`）并在
主机附着前完成初始化。信息头地址取自 `--elf` 的 `EventRecorderInfo` 符号或
`--address`。

```bash
cmsis-dap-cli --target STM32F030C8 --elf firmware.axf \
  evr monitor --ctx 0,2 --count 0 --log-dir logs
```

### 监控输出、时间戳与日志导出

`watch`、`rtt monitor`、`evr monitor` 的每一行都带主机采集时间戳
`[YYYY-MM-DD HH:MM:SS.mmm]`。`--json` 下每个采样/事件是 stdout 上的一行
NDJSON，并带 `host_ts` 字段（RFC 3339，毫秒 + 时区）；EVR 事件额外保留
目标侧 `timestamp_ticks`/`timestamp_secs`。

监控输出默认同时写入日志文件，位置是当前目录，文件名为自动生成
（`watch-<unix秒>.log`、`rtt-<unix秒>.log`、`evr-<unix秒>.log`）；
`--log-dir DIR` 指定其他目录（不存在会自动创建），`--log-file FILE` 则追加
写入确切文件。文件内容与 stdout 完全一致（每采样/事件一行），每行立即 flush；
监控启动时在 stderr 打印 `logging to <路径>`。

### 交互式 shell

```text
repl
```

启动 J-Link Commander 风格 shell（见[REPL](#repl)）。

## 从 FLM 生成 target YAML

对于 probe-rs 内置库没有的芯片，烧录需要一份描述芯片并内嵌厂商 Flash 算法
的 target YAML。不用手写——`chip generate` 读取 Keil FLM，你只需要提供
Flash 与 SRAM 地址范围：

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
`--output -` 把 YAML 打印到 stdout。

然后连接：

```bash
cmsis-dap-cli --target-yaml MYCHIP.yaml connect
```

当 target YAML 只定义一颗芯片变体时，`--target` 可以省略，CLI 会自动选择；
若定义了多颗，则必须给 `--target NAME`（命令会提示可用的名字）。

生成的 YAML 会把算法放在 `SRAM 起始 + 0x20`；请确保 SRAM 范围足够大
（放不下时命令会拒绝生成）。

## 查看与搜索芯片

```bash
cmsis-dap-cli chip list
cmsis-dap-cli chip search STM32F103
cmsis-dap-cli chip search stm32f103c8
cmsis-dap-cli --target-yaml MYCHIP.yaml chip search MYCHIP
```

搜索不区分大小写、按子串匹配。加 `--json` 会输出完整信息（所属系列、内核、
Flash 与 RAM 范围），方便脚本处理。

## 示例

### 一次完整调试会话

```bash
cmsis-dap-cli list
cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 connect
cmsis-dap-cli --target STM32F030C8 read --address 0x20000000 --width u32 --count 4
cmsis-dap-cli --target STM32F030C8 halt
cmsis-dap-cli --target STM32F030C8 reg get pc
cmsis-dap-cli --target STM32F030C8 step
cmsis-dap-cli --target STM32F030C8 resume
```

### 烧录固件并校验

```bash
cmsis-dap-cli --target STM32F030C8 flash erase --address 0x08000000 --size 0x10000
cmsis-dap-cli --target STM32F030C8 flash program --address 0x08000000 --file fw.hex --verify
cmsis-dap-cli --target STM32F030C8 read --address 0x08000000 --width u8 --count 0x100 --output dump.bin --format bin
```

### 脚本文件

`flash.jlink`：

```text
connect
halt
reg pc
savebin C:/dump.bin 0x20000000 0x100
resume
q
```

```bash
cmsis-dap-cli --target STM32F030C8 script --file flash.jlink
```

### 机器可读输出

```bash
cmsis-dap-cli --json connect
cmsis-dap-cli --json read --address 0x20000000 --width u32 --count 2
```

```json
{"target":{"core_type":"Armv6m","core_count":1,"ap_count":1, ...}}
{"address":536870912,"width":"u32","values":[64000000,1]}
```

## 输出与退出码

- 默认输出人类可读；`--json` 输出与 MCP 工具一致的 structured payload。
  日志始终写 stderr。
- 退出码：`0` 成功，`1` 运行时错误（探针/连接/烧录失败），`2` 用法错误
  （未知参数、非法取值、缺参）。
- 监控命令（`watch`、`rtt monitor`、`evr monitor`）每采样/事件输出一行
  （`--json` 为 NDJSON），Ctrl-C 干净停止后退出 `0`；`--count N` 限定轮数，
  便于脚本与 CI。

## REPL

`repl` 启动交互式 shell，一个会话保持打开，halt/读/恢复可以跨行执行：

```text
$ cmsis-dap-cli --probe-id 0123456789AB --target STM32F030C8 repl
cmsis-dap-cli> connect
target: {"ap_count":1,"core_count":1,"core_type":"Armv6m", ...}
cmsis-dap-cli> halt
halted: true
cmsis-dap-cli> reg pc
pc = 0x800122A
cmsis-dap-cli> resume
running: true
cmsis-dap-cli> q
```

`?`/`help` 显示支持的命令；`q`/`exit` 退出。REPL 继承全局连接参数，`connect`
直接使用它们（不用重敲 `--target`）。REPL 里 Flash 擦除/烧录同样直接执行。

REPL 还提供带持久观察状态的实时调试命令：

```text
watch add <name|0xADDR> [--width u8|u16|u32|u64] [--label TEXT]
watch list | watch remove <idx|name> | watch clear
watch interval <ms>
watch run [--count N] [--log-dir DIR | --log-file FILE]
rtt [info] [--channel 0,1] [--count N] [--interval-ms N] [--log-dir DIR | --log-file FILE]
evr [info] [--ctx 0..7] [--count N] [--log-dir DIR | --log-file FILE]
```

监控命令运行到 Ctrl-C（或 `--count N`）后回到提示符。

## 脚本命令

脚本引擎（`script` 与 REPL 共用）支持：

```text
connect | disconnect | init        会话管理
si swd|jtag                        接口
speed <khz>                        时钟
device <name>                      目标芯片
adapter serial <id>                选择探针
halt | go | step                   执行控制
reset [run|halt]                   复位
reg <name> [<value>] | regs        内核寄存器
mem8/16/32 <addr> [<n>] | mdb/mdh/mdw   读内存
w8/16/32 <addr> <value> | mwb/mwh/mww   写内存
savebin <file> <addr> <size>       导出内存到二进制文件
dump_image <file> <addr> <size>    savebin 别名
loadbin <file> <addr>              烧录二进制文件
loadfile <file> [<addr>]           烧录 axf/elf/bin/hex
flash write_image <file> [<addr>]  loadfile 别名
flash erase_sector <addr> <size>   擦除一段 Flash
erase                              全片擦除
verifybin <file> [<addr>]          用文件校验内存
verify_image <file> [<addr>]       verifybin 别名
sleep <ms> | echo <text>           辅助命令
targets                            显示已连接目标
? | help | q | exit                帮助与退出
```

## 常见问题与提示

- **选择芯片**：内置芯片（`chip search NAME`）直接 `--target NAME` 即可；
  其他芯片先用 `chip generate` 生成一次 target YAML，再用 `--target-yaml`
  加载（单变体自动选择；多变体需 `--target`）。
- **Flash 需要芯片定义**：没有定义时擦除/烧录会明确报错，而不是静默无效果。
- **读寄存器需要先暂停内核**：一次性模式下用 `script`/`repl`，让 `halt` 和
  `reg` 共享同一会话。
- **Flash 不能用 `write` 写**：直接写 Flash 地址会被拒绝；用 `flash program`。
- **数字格式**：十进制或十六进制（`0x...`）均可。
