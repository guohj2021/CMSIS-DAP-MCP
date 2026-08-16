# CLI

`cmsis-dap-cli` is a standalone command-line tool for humans and scripts. It
shares the same `cmsis-dap-core` engine as the MCP server (probe enumeration,
memory, core control, SVD, flash and scripting), but talks to you directly
instead of over MCP.

## Install

Zero-install via npm:

```bash
npx -y cmsis-dap-cli --help
```

Or download a native binary for Windows / Linux / macOS from
[GitHub Releases](https://github.com/guohj2021/CMSIS-DAP-MCP/releases).

## Command overview

Global options (before the subcommand): `--probe-id`, `--protocol swd|jtag`,
`--speed-khz`, `--target`, `--under-reset`, `--target-yaml`, `--svd`, `--json`,
`--log-level`, `--log-file`.

| Command | Purpose |
| --- | --- |
| `list` | enumerate connected probes |
| `info` | show probe information |
| `connect` / `disconnect` / `target` | manage the session and show target info |
| `read --address A --width W --count N [--output FILE --format bin\|hex]` | read memory or export a range to a file |
| `write --address A --width W --values V1,V2,...` | write memory |
| `verify --address A --width W --values ...` | compare memory against expected values |
| `regs` / `reg get NAME\|NUM` / `reg set NAME\|NUM VALUE` | core register access |
| `status` / `halt` / `resume` / `step` / `reset [--mode run\|halt]` | execution control |
| `bp set ADDR` / `bp list` / `bp clear` | hardware breakpoints |
| `wp set ADDR --access read\|write\|rw` / `wp list` / `wp clear` | watchpoints |
| `dap read ADDR` / `dap write ADDR VALUE` | raw DAP (DP/AP) access |
| `svd list` / `svd read PERIPH.REG[.FIELD]` / `svd write PERIPH.REG[.FIELD] VALUE` | SVD named access (requires `--svd FILE`) |
| `flash erase --address A --size N` / `flash program --address A --file F [--format elf\|axf\|bin\|hex] [--verify]` | flash erase / program |
| `script --file F` or `--text TEXT` | run a J-Link / OpenOCD style script |
| `chip generate --flm F --flash-start A --flash-size N --sram-start A --sram-size N [--name NAME] [--output FILE]` | generate a probe-rs target YAML from a Keil FLM |
| `chip list` / `chip search KEYWORD` | list or search chip variants (built-in plus `--target-yaml` chips) |
| `repl` | interactive shell (J-Link Commander style) |

Commands that need a target auto-connect using the global connection options.
Numbers may be written in decimal or hex (`0x...`).

## Examples

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

In the REPL, `?`/`help` shows the supported J-Link / OpenOCD style commands and
`q`/`exit` quits.

## Generating a target YAML from an FLM

For chips that are not built into probe-rs, flashing needs a target YAML that
describes the chip and embeds the vendor flash algorithm. You do not have to
hand-write it: `chip generate` reads a Keil FLM file (the vendor flash
algorithm) and needs only the Flash and SRAM address ranges from you:

```bash
cmsis-dap-cli chip generate \
  --flm MyChip_64.FLM \
  --flash-start 0x08000000 --flash-size 0x10000 \
  --sram-start 0x20000000 --sram-size 0x2000 \
  --name MYCHIP --output MYCHIP.yaml
```

Everything else is extracted from the FLM automatically: the algorithm
instructions, the entry-point offsets (`Init`/`ProgramPage`/`EraseSector`/
`EraseChip`), the static data base, the FlashDevice descriptor (page size,
erased value, sector size, timeouts) and the device name. `--name` defaults to
the FLM file stem; use `--output -` to print the YAML to stdout. Then load it
with the same tool:

```bash
cmsis-dap-cli --target-yaml MYCHIP.yaml --target MYCHIP connect
```

When the target YAML defines exactly one chip variant, `--target` can be
omitted — the CLI auto-selects it. If it defines several variants, `--target
NAME` is required (the command lists the available names).

The generated YAML places the algorithm at `SRAM start + 0x20`; make sure the
SRAM range you provide is large enough for the algorithm (the command refuses
to emit a YAML whose algorithm does not fit).

## Listing and searching chips

To see which chips are known (and thus usable with `--target` or the REPL
`device` command), list or search the probe-rs built-in database:

```bash
cmsis-dap-cli chip list
cmsis-dap-cli chip search STM32F103
cmsis-dap-cli chip search stm32f103c8
```

The search is case-insensitive and matches substrings of chip names. Pass
`--target-yaml FILE` to include custom chips generated with `chip generate` in
the listing; `--json` returns the full details (family, cores, flash and RAM
ranges) for scripting.

## Output and exit codes

- Default output is human-readable. Add `--json` for machine-readable JSON that
  mirrors the MCP tool payloads. Logs always go to stderr.
- Exit codes: `0` success, `1` runtime error, `2` usage error.

## Flash operations

`flash erase` and `flash program` run directly in the CLI (including from
`script` and the `repl`); there is no approval prompt or `--yes` flag. They
still require a target that defines flash: connect with `--target-yaml`/
`--target` (or `device NAME` + `connect` in a script/REPL), otherwise the
operation fails with a clear error instead of silently doing nothing.

Flash erasing and programming can permanently damage a device. Double-check
the address and file before running them.
