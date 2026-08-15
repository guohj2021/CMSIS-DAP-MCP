# 脚本使用

`run_script` 用 J-Link Commander / OpenOCD 风格命令子集执行线性调试脚本，
适合把"连接→读内存→烧录文件→复位"等重复流程固化成脚本，不必逐条调用工具。

## 运行脚本

传脚本文件路径，或内联文本：

```text
run_script { "path": "/path/to/demo.jlink" }
run_script { "script": "halt\nreg pc\nresume" }
```

`path` 与 `script` 二选一。脚本顺序执行，遇到第一条失败命令即停止。返回
`ok`、命令数量与每条命令的结果：

```json
{
  "ok": true,
  "commands": 3,
  "results": [
    { "command": "halt", "status": "ok", "output": { "halted": true } },
    { "command": "reg pc", "status": "ok", "output": { "register": "pc", "value": 134228884 } },
    { "command": "resume", "status": "ok", "output": { "running": true } }
  ]
}
```

## 语法

- 每行一条命令；`;` 也可作分隔符（OpenOCD 风格）。
- 注释以 `//` 或 `#` 开头。
- 参数可用 `"..."` 或 `'...'` 引起来（路径含空格时）。
- 数字支持十进制或 `0x` 十六进制。
- `sleep <ms>` 延时；`echo <text>` 输出；`q` / `exit` 结束脚本。

## 命令参考

以 J-Link Commander 命令名为主，OpenOCD 别名映射到同一批操作。

| 分类 | J-Link | OpenOCD 别名 | 说明 |
| --- | --- | --- | --- |
| 会话 | `connect`、`si SWD\|JTAG`、`speed <khz>`、`device <name>`、`disconnect` | `init`、`adapter speed <khz>`、`adapter serial <serial>`、`targets` | 连接/配置会话 |
| 内核 | `halt`、`go`、`step`、`reset [halt\|run]`、`reg <name> [value]`、`regs` | `resume`、`reset`、`reg <name> [value]` | 执行控制 |
| 内存 | `mem8/16/32 <addr> [count]`、`w8/16/32 <addr> <value>` | `mdb/mdh/mdw`、`mwb/mwh/mww` | 读写内存 |
| 文件 | `savebin <path> <addr> <size>`、`loadbin <path> <addr>`、`loadfile <path> [addr]`、`verifybin <path> <addr>` | `dump_image <path> <addr> <size>`、`flash write_image <path> [offset]`、`verify_image <path> [offset]` | 导出/烧录/校验文件 |
| Flash | `erase` | `flash erase_sector <addr> <size>` | 擦除 Flash |
| 其他 | `sleep <ms>`、`echo <text>`、`q` / `exit` | - | 工具命令 |

`savebin` / `dump_image` 导出原始二进制；`loadbin` 按给定地址烧录原始二进制；
`loadfile` / `flash write_image` 烧录文件（按扩展名推断 elf/axf/bin/hex）；
`verifybin` / `verify_image` 把文件与目标内存比较。

## 示例

保存 Flash 前 16KB，然后烧录并校验新固件：

```text
connect
savebin C:/dump/fw.bin 0x08000000 0x4000
loadbin C:/fw/new.bin 0x08000000
verifybin C:/fw/new.bin 0x08000000
reset halt
go
q
```

OpenOCD 风格内联脚本：

```text
halt; mdw 0x20000000 4; reg pc; resume
```

烧录 HEX 文件：

```text
connect
flash write_image C:/fw/out.hex
reset
```

## 安全

`run_script` 是写级工具。脚本内的破坏性命令（`erase`、`loadbin`、
`loadfile`、`flash write_image`、`flash erase_sector`）仍要求服务器以
`--allow-destructive` 启动，否则返回 `DestructiveDisabled`。
