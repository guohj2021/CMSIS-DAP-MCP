# 工具参考

安全等级：**读**（始终可用）、**写**（由客户端审批）、**破坏性**（需
`--allow-destructive`）。

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
