# Tools

Levels: **Read** (always available), **Write** (governed by client approval),
**Destructive** (requires `--allow-destructive` at startup **or**
`update_config` with `allow_destructive: true` at runtime).

## Probe and session

| Tool | Params | Level |
| --- | --- | --- |
| `list_probes` | - | Read |
| `get_probe_info` | `probe_id` (optional) | Read |
| `connect` | `probe_id`, `protocol` (`swd`/`jtag`, default `swd`), `speed_khz`, `target`, `under_reset` | Write |
| `disconnect` | - | Write |
| `get_target_info` | - | Read |

`list_probes` returns the probe id, vendor/product, serial, product id,
interface, HID flag, supported protocols, speed and target voltage (when the
probe reports it).

`get_target_info` returns the core type and count, the real AP count, CPUID,
DPIDR and a memory map summary (RAM/NVM regions).

## Memory

| Tool | Params | Level |
| --- | --- | --- |
| `read_memory` | `address`, `width` (`u8`/`u16`/`u32`/`u64`), `count` (default 1) | Read |
| `write_memory` | `address`, `width`, `values` | Write |
| `verify_memory` | `address`, `width`, `data` | Read |

`verify_memory` reads back the given range and compares it with `data`,
returning `verified` and a list of `mismatches`.

`read_memory` can also export a range to a file: pass `path` plus
`format` (`bin` default or `hex`) and `count` becomes the number of **bytes**
to read. Example:

```text
read_memory { "address": 0x08000000, "width": "u8", "count": 0x1000, "path": "firmware.bin", "format": "bin" }
```

## Core

| Tool | Params | Level |
| --- | --- | --- |
| `read_core_register` | `name` **or** `number` | Read |
| `write_core_register` | `name` **or** `number`, `value` | Write |
| `list_core_registers` | - | Read |
| `get_core_status` | - | Read |
| `halt` | - | Write |
| `resume` | - | Write |
| `step` | - | Write |
| `reset` | `mode` (`run` default / `halt`) | Write |

Register names are resolved case-insensitively. Special roles (`pc`, `sp`,
`fp`, `lr`/`ra`, `psr`/`xpsr`, `msp`, `psp`, `fpsr`) and general registers
(`r0`-`r15`) are supported; any other name is looked up in the architecture
register file. `list_core_registers` returns all available names.

`get_core_status` returns `state` (`running`/`halted`/`sleeping`/`locked_up`/
`unknown`), the `halt_reason` when halted, and the program counter when
halted.

## Non-invasive debugging

| Tool | Params | Level |
| --- | --- | --- |
| `dump_cpu_state` | `address` (repeatable, `0xADDR` or ELF symbol), `stack_words` (optional), `no_restore` (optional) | Read |

`dump_cpu_state` takes a CPU snapshot **without ever resetting** the target:
core registers (read during a short halt), Cortex-M fault status registers
(CFSR/HFSR/DFSR/MMFAR/BFAR, read without halting), the top words of the
MSP/PSP stacks and optional memory samples at the given addresses. By default
the previous run state is restored afterwards; pass `no_restore: true` to
leave the core halted. Addresses accept `0xADDR` or ELF symbol names (when
the server is started with an `--elf` file).

## Breakpoints and watchpoints

| Tool | Params | Level |
| --- | --- | --- |
| `set_breakpoint` | `address` | Write |
| `clear_breakpoints` | - | Write |
| `list_breakpoints` | - | Read |
| `set_watchpoint` | `address`, `access` (`read`/`write`/`rw`) | Write |
| `clear_watchpoints` | - | Write |
| `list_watchpoints` | - | Read |

Watchpoints use the core's DWT comparators. They trigger on core load/store
accesses, not on debugger writes. If the target has no DWT comparators, the
server returns `UnsupportedFeature`.

## DAP

| Tool | Params | Level |
| --- | --- | --- |
| `read_dap` | `address` | Read |
| `write_dap` | `address`, `value` | Write |

DAP addresses use APSEL in bits 24-31 for AP access (e.g. `0x010000FC`);
otherwise bits 0-7 are the DP register address (bits 4-7 select the DP bank).

## SVD

| Tool | Params | Level |
| --- | --- | --- |
| `load_svd` | `path` | Write |
| `list_peripherals` | - | Read |
| `read_peripheral` | `peripheral`, `register`, `field` (optional) | Read |
| `write_peripheral` | `peripheral`, `register`, `field` (optional), `value` | Write |

Field writes are read-modify-write.

## Flash

| Tool | Params | Level |
| --- | --- | --- |
| `erase_flash` | `address`, `size` | Destructive |
| `program_flash` | `address`, `data` **or** `path`, `format` (optional), `verify` (optional) | Destructive |

`erase_flash` erases only the sectors overlapping `[address, address+size)`;
pass the full flash range to erase the whole chip. `program_flash` with
`verify: true` reads the data back after programming. Instead of raw `data`
you can pass a firmware file via `path`:

```text
program_flash { "address": 0x08004000, "path": "/path/to/fw.hex", "format": "hex", "verify": true }
```

Supported formats: `elf`, `axf` (same container as ELF), `bin` (requires
`address`), `hex`/`ihex`/`intelhex`, or `auto` (default, inferred from the
file extension `.elf`/`.axf`/`.bin`/`.hex`/`.ihx`).

## Chip definition

| Tool | Params | Level |
| --- | --- | --- |
| `define_chip` | `flm`, `flash_start`, `flash_size`, `sram_start`, `sram_size`, `core` (optional, default `armv6m`), `name` (optional, default FLM file stem) | Write |

`define_chip` registers a custom/unknown chip at runtime from a Keil FLM
flash algorithm file — no standalone probe-rs CLI or pre-built target YAML
is needed. The FLM is parsed to extract the flash algorithm (code, entry
points, page size, sector layout, erased value, timeouts), and a probe-rs
target YAML is generated and registered in the running server's backend
registry. After registration, call `connect` with `target` set to the chip
name (or omit it when only one variant is defined) to attach.

Parameters:

- `flm` — path to a Keil FLM file (ARM ELF containing the vendor flash
  algorithm and a `FlashDevice` descriptor).
- `flash_start` / `flash_size` — Flash memory address range (e.g.
  `0x08000000` / `0x10000` for 64 KB). The FLM descriptor's own values are
  unreliable, so you must supply these explicitly.
- `sram_start` / `sram_size` — SRAM address range (e.g. `0x20000000` /
  `0x2000` for 8 KB). The FLM does not contain this information.
- `core` — ARM architecture profile: `armv6m` (Cortex-M0/M0+, default),
  `armv7m` (Cortex-M3), or `armv7em` (Cortex-M4/M7).
- `name` — chip/variant name used with `connect`. Defaults to the FLM file
  stem.

Example:

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

## Runtime configuration

| Tool | Params | Level |
| --- | --- | --- |
| `get_config` | - | Read |
| `update_config` | `allow_destructive` (optional), `tcp_port` (optional), `gdb_port` (optional) | Write |
| `reload_config` | - | Write |

These tools manage the server's runtime configuration. The server can be
started with zero arguments (to-be-configured state) and fully configured
at runtime — no restart needed.

`get_config` returns the current configuration as JSON:
`allow_destructive`, `tcp_port`, `gdb_port`, `config_file`.

`update_config` applies a partial update: omit any field to keep its
current value. The candidate config is validated *before* anything is
written, so an invalid value rejects the whole update atomically (no
partial apply). After a successful update, the server reconciles its
running TCP/GDB tasks to match the new config (idempotent).

- `allow_destructive` — `true` enables `erase_flash` / `program_flash` and
  destructive script commands; `false` disables them.
- `tcp_port` — set to a port number (1–65535) to start or move the remote
  JSON-RPC TCP server on `127.0.0.1`; set to `null` to stop it.
- `gdb_port` — set to a port number to start the GDB server. A running GDB
  server **cannot** be moved at runtime; restart the server to change its
  port.

`reload_config` re-reads the config file supplied at startup via
`--config-file` and applies it. Fails with a clear error when no file was
provided, the file is missing, or the contents are invalid.

Example:

```text
get_config
  -> {"allow_destructive": false, "tcp_port": null, "gdb_port": null, "config_file": null}

update_config { "allow_destructive": true, "tcp_port": 4000 }
  -> {"allow_destructive": true, "tcp_port": 4000, "gdb_port": null, "config_file": null}
```

## Scripts

| Tool | Params | Level |
| --- | --- | --- |
| `run_script` | `path` **or** `script` | Write |

`run_script` executes a linear debug script using a J-Link Commander /
OpenOCD style command subset. See [Scripting](./scripting.md) for the full
command reference and examples.

## Error codes

Errors return structured JSON with `code` and `message`:
`ProbeNotFound`, `ConnectFailed`, `NotConnected`, `ProtocolError`, `Timeout`,
`MemoryFault`, `SvdNotLoaded`, `FileError`, `UnsupportedFeature`,
`DestructiveDisabled`, `InvalidArgument`, `InternalError`.
