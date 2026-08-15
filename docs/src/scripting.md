# Scripting

`run_script` executes a linear debug script with a J-Link Commander / OpenOCD
style command subset. It is useful for repeatable workflows such as connect,
dump memory, program a file and reset, without issuing each tool call
separately.

## Running a script

Provide a script file path, or inline text:

```text
run_script { "path": "/path/to/demo.jlink" }
run_script { "script": "halt\nreg pc\nresume" }
```

Exactly one of `path` and `script` is required. Scripts run sequentially and
stop on the first failing command. The result contains `ok`, the number of
commands, and one result per command:

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

## Syntax

- One command per line; `;` is also accepted as a separator (OpenOCD style).
- Comments start with `//` or `#`.
- Arguments can be quoted with `"..."` or `'...'` (paths with spaces).
- Numbers accept decimal or `0x` hexadecimal.
- `sleep <ms>` pauses; `echo <text>` prints; `q` / `exit` stops the script.

## Command reference

J-Link Commander names are primary; OpenOCD aliases map to the same
operations.

| Area | J-Link | OpenOCD alias | Meaning |
| --- | --- | --- | --- |
| Session | `connect`, `si SWD\|JTAG`, `speed <khz>`, `device <name>`, `disconnect` | `init`, `adapter speed <khz>`, `adapter serial <serial>`, `targets` | Connect / configure session |
| Core | `halt`, `go`, `step`, `reset [halt\|run]`, `reg <name> [value]`, `regs` | `resume`, `reset`, `reg <name> [value]` | Execution control |
| Memory | `mem8/16/32 <addr> [count]`, `w8/16/32 <addr> <value>` | `mdb/mdh/mdw`, `mwb/mwh/mww` | Read/write memory |
| Files | `savebin <path> <addr> <size>`, `loadbin <path> <addr>`, `loadfile <path> [addr]`, `verifybin <path> <addr>` | `dump_image <path> <addr> <size>`, `flash write_image <path> [offset]`, `verify_image <path> [offset]` | Export / program / verify files |
| Flash | `erase` | `flash erase_sector <addr> <size>` | Erase flash |
| Misc | `sleep <ms>`, `echo <text>`, `q` / `exit` | - | Utility commands |

`savebin` / `dump_image` export raw binary. `loadbin` programs a raw binary
at the given address. `loadfile` / `flash write_image` program a file whose
format is inferred from the extension (elf/axf/bin/hex). `verifybin` /
`verify_image` compare a file with target memory.

## Examples

Save the first 16 KB of flash, then program a binary and verify it:

```text
connect
savebin C:/dump/fw.bin 0x08000000 0x4000
loadbin C:/fw/new.bin 0x08000000
verifybin C:/fw/new.bin 0x08000000
reset halt
go
q
```

OpenOCD style inline script:

```text
halt; mdw 0x20000000 4; reg pc; resume
```

Program a HEX file:

```text
connect
flash write_image C:/fw/out.hex
reset
```

## Security

`run_script` is a write-level tool. Destructive commands inside a script
(`erase`, `loadbin`, `loadfile`, `flash write_image`, `flash erase_sector`)
additionally require the server to be started with `--allow-destructive`;
otherwise they fail with `DestructiveDisabled`.
