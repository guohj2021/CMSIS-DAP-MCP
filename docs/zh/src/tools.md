# 工具参考

安全等级：**读**（始终可用）、**写**（由客户端审批）、**破坏性**（需
启动时 `--allow-destructive` **或** 运行时 `update_config` 设
`allow_destructive: true`）。

## 探针与会话

| 工具 | 参数 | 等级 |
| --- | --- | --- |
| `list_probes` | - | 读 |
| `get_probe_info` | `probe_id`（可选） | 读 |
| `connect` | `probe_id`、`protocol`（`swd`/`jtag`，默认 `swd`）、`speed_khz`、`target`、`under_reset` | 写 |
| `disconnect` | - | 写 |
| `get_target_info` | - | 读 |

`list_probes` 返回探针 id、厂商/产品、序列号、产品 id、接口、HID 标记、
支持的协议、速度与目标电压（探针支持时）。

`get_target_info` 返回内核类型与数量、真实 AP 数量、CPUID、DPIDR 与内存映射
摘要（RAM/NVM 区域）。

## 内存

| 工具 | 参数 | 等级 |
| --- | --- | --- |
| `read_memory` | `address`、`width`（`u8`/`u16`/`u32`/`u64`）、`count`（默认 1）、`path`、`format` | 读 |
| `write_memory` | `address`、`width`、`values` | 写 |
| `verify_memory` | `address`、`width`、`data` | 读 |

`verify_memory` 读回指定范围并与 `data` 比较，返回 `verified` 与 `mismatches`
列表。

`read_memory` 还可以把范围导出到文件：传 `path` 加 `format`（默认 `bin` 或
`hex`），此时 `count` 表示**字节数**。示例：

```text
read_memory { "address": 0x08000000, "width": "u8", "count": 0x1000, "path": "firmware.bin", "format": "bin" }
```

## 内核

| 工具 | 参数 | 等级 |
| --- | --- | --- |
| `read_core_register` | `name` **或** `number` | 读 |
| `write_core_register` | `name` **或** `number`、`value` | 写 |
| `list_core_registers` | - | 读 |
| `get_core_status` | - | 读 |
| `halt` | - | 写 |
| `resume` | - | 写 |
| `step` | - | 写 |
| `reset` | `mode`（默认 `run` / `halt`） | 写 |

寄存器名大小写不敏感。支持特殊角色（`pc`、`sp`、`fp`、`lr`/`ra`、
`psr`/`xpsr`、`msp`、`psp`、`fpsr`）与通用寄存器（`r0`-`r15`）；其他名称会在
架构寄存器表中查找。`list_core_registers` 返回全部可用名称。

`get_core_status` 返回 `state`（`running`/`halted`/`sleeping`/`locked_up`/
`unknown`）、暂停时的 `halt_reason` 与程序计数器。

## 非侵入调试

| 工具 | 参数 | 等级 |
| --- | --- | --- |
| `dump_cpu_state` | `address`（可重复，`0xADDR` 或 ELF 符号）、`stack_words`（可选）、`no_restore`（可选） | 读 |

`dump_cpu_state` 在**永不复位**目标的前提下采集 CPU 快照：内核寄存器（在短暂停机时读取）、Cortex-M fault 状态寄存器（CFSR/HFSR/DFSR/MMFAR/BFAR，不停机读取）、MSP/PSP 栈顶字与按给定地址的可选内存采样。默认读取后恢复原运行状态；传入 `no_restore: true` 则保持核心停机。地址接受 `0xADDR` 或 ELF 符号名（当服务器以 `--elf` 文件启动时）。

## 断点与数据观察点

| 工具 | 参数 | 等级 |
| --- | --- | --- |
| `set_breakpoint` | `address` | 写 |
| `clear_breakpoints` | - | 写 |
| `list_breakpoints` | - | 读 |
| `set_watchpoint` | `address`、`access`（`read`/`write`/`rw`） | 写 |
| `clear_watchpoints` | - | 写 |
| `list_watchpoints` | - | 读 |

数据观察点使用内核的 DWT 比较器，只对内核的读写访问触发，不会因调试器写入
触发。目标没有 DWT 比较器时返回 `UnsupportedFeature`。

## DAP

| 工具 | 参数 | 等级 |
| --- | --- | --- |
| `read_dap` | `address` | 读 |
| `write_dap` | `address`、`value` | 写 |

DAP 地址在 bit 24-31 放 APSEL 表示 AP 访问（例如 `0x010000FC`）；否则 bit 0-7
是 DP 寄存器地址（bit 4-7 选择 DP bank）。

## SWO / SWV 追踪

| 工具 | 参数 | 等级 |
| --- | --- | --- |
| `start_swo` | `baud`、`tpiu_clk` | 写 |
| `stop_swo` | - | 写 |
| `read_swo` | `max_bytes`（可选） | 读 |

`start_swo` 配置 TPIU/SWO 输出（`tpiu_clk` 为 TPIU 时钟频率，`baud` 为 SWO
波特率）并开启追踪；`read_swo` 返回可用的原始字节（hex 编码，`bytes` +
`data_hex`）；`stop_swo` 关闭追踪。SWO 需要目标把 trace 数据路由到 SWO 引脚，
且探针支持 SWO 采集。

## SVD

| 工具 | 参数 | 等级 |
| --- | --- | --- |
| `load_svd` | `path` | 写 |
| `list_peripherals` | - | 读 |
| `read_peripheral` | `peripheral`、`register`、`field`（可选） | 读 |
| `write_peripheral` | `peripheral`、`register`、`field`（可选）、`value` | 写 |

位域写入为读-改-写。

## Flash

| 工具 | 参数 | 等级 |
| --- | --- | --- |
| `erase_flash` | `address`、`size` | 破坏性 |
| `program_flash` | `address`、`data` **或** `path`、`format`（可选）、`verify`（可选） | 破坏性 |

`erase_flash` 只擦除与 `[address, address+size)` 重叠的扇区；传入完整 Flash
范围即整片擦除。`program_flash` 带 `verify: true` 时烧写后读回校验。除了原始
`data`，还可以用 `path` 传入固件文件：

```text
program_flash { "address": 0x08004000, "path": "/path/to/fw.hex", "format": "hex", "verify": true }
```

支持的格式：`elf`、`axf`（与 ELF 同容器）、`bin`（必须给 `address`）、
`hex`/`ihex`/`intelhex`，或 `auto`（默认，按扩展名
`.elf`/`.axf`/`.bin`/`.hex`/`.ihx` 推断）。

## 选项字节（Option bytes）

| 工具 | 参数 | 等级 |
| --- | --- | --- |
| `read_option_bytes` | - | 读 |
| `write_option_bytes` | `bytes`（`{name, address, value}` 数组） | 破坏性 |

选项字节是芯片配置字段（STM32：RDP、USER、DATA0、DATA1，位于 FLASH_OPTCR），
通过原始 DAP 寄存器访问。布局因芯片系列而异，以 STM32 布局为参考实现。
`write_option_bytes` 具有破坏性（可能改变读保护或锁死设备），需要
`--allow-destructive`。

## 芯片定义

| 工具 | 参数 | 等级 |
| --- | --- | --- |
| `define_chip` | `flm`、`flash_start`、`flash_size`、`sram_start`、`sram_size`、`core`（可选，默认 `armv6m`）、`name`（可选，默认 FLM 文件名） | 写 |

`define_chip` 在运行时从 Keil FLM 闪存算法文件注册自定义/未知芯片——
无需独立 probe-rs CLI 或预构建 target YAML。FLM 被解析以提取闪存算法
（代码、入口点、页大小、扇区布局、擦除值、超时），生成 probe-rs target YAML
并注册到运行中服务器的 backend registry。注册后，调用 `connect` 并将
`target` 设为芯片名（仅定义一个变体时可省略）即可连接。

参数：

- `flm` — Keil FLM 文件路径（ARM ELF，含厂商闪存算法与 `FlashDevice` 描述符）。
- `flash_start` / `flash_size` — Flash 内存地址范围（如 `0x08000000` /
  `0x10000` 表示 64 KB）。FLM 描述符自身的值不可靠，必须显式提供。
- `sram_start` / `sram_size` — SRAM 地址范围（如 `0x20000000` /
  `0x2000` 表示 8 KB）。FLM 不包含此信息。
- `core` — ARM 架构 profile：`armv6m`（Cortex-M0/M0+，默认）、`armv7m`
  （Cortex-M3）、`armv7em`（Cortex-M4/M7）。
- `name` — 用于 `connect` 的芯片/变体名。默认取 FLM 文件名（去掉扩展名）。

示例：

```text
define_chip {
  "flm": "C:/SDK/Libraries/Flash/MyChip_64.FLM",
  "flash_start": 0x08000000, "flash_size": 0x10000,
  "sram_start": 0x20000000, "sram_size": 0x2000,
  "core": "armv6m", "name": "MyChip"
}
connect { "target": "MyChip", "protocol": "swd" }
load_svd { "path": "C:/SDK/SVD/MyChip.svd" }
erase_flash { "address": 0x0800FC00, "size": 0x400 }
program_flash { "address": 0x0800FC00, "data": [0xDE, 0xAD, 0xBE, 0xEF], "verify": true }
```

## 运行时配置

| 工具 | 参数 | 等级 |
| --- | --- | --- |
| `get_config` | - | 读 |
| `update_config` | `allow_destructive`（可选）、`tcp_port`（可选）、`gdb_port`（可选） | 写 |
| `reload_config` | - | 写 |

这些工具管理服务器的运行时配置。服务器可以零参数启动（待配置态），然后
完全在运行时配置——无需重启。

`get_config` 返回当前配置的 JSON：`allow_destructive`、`tcp_port`、
`gdb_port`、`config_file`。

`update_config` 执行部分更新：省略任意字段即保持当前值。候选配置在写入前
先校验，无效值将整体拒绝（原子性，不部分生效）。更新成功后，服务器自动
收敛运行中的 TCP/GDB 任务以匹配新配置（幂等）。

- `allow_destructive` — `true` 开启 `erase_flash` / `program_flash` 及
  破坏性脚本命令；`false` 关闭。
- `tcp_port` — 设为端口号（1–65535）启动或迁移 `127.0.0.1` 上的远程
  JSON-RPC TCP 服务器；设为 `null` 停止。
- `gdb_port` — 设为端口号启动 GDB 服务器。已运行的 GDB 服务器**无法**
  运行时迁移端口；需重启服务器才能改端口。

`reload_config` 重新读取启动时通过 `--config-file` 指定的配置文件并应用。
未提供文件、文件缺失或内容无效时返回明确错误。

示例：

```text
get_config
  -> {"allow_destructive": false, "tcp_port": null, "gdb_port": null, "config_file": null}

update_config { "allow_destructive": true, "tcp_port": 4000 }
  -> {"allow_destructive": true, "tcp_port": 4000, "gdb_port": null, "config_file": null}
```

## 脚本

| 工具 | 参数 | 等级 |
| --- | --- | --- |
| `run_script` | `path` **或** `script` | 写 |

`run_script` 用 J-Link Commander / OpenOCD 风格命令子集执行线性调试脚本。
完整命令参考与示例见 [脚本使用](./scripting.md)。

## 错误码

错误返回结构化 JSON，含 `code` 与 `message`：`ProbeNotFound`、
`ConnectFailed`、`NotConnected`、`ProtocolError`、`Timeout`、`MemoryFault`、
`SvdNotLoaded`、`FileError`、`UnsupportedFeature`、`DestructiveDisabled`、
`InvalidArgument`、`InternalError`。
