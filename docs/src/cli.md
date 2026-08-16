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
`--speed-khz`, `--target`, `--under-reset`, `--target-yaml`, `--svd`, `--yes`,
`--json`, `--log-level`, `--log-file`.

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
cmsis-dap-cli --yes flash program --address 0x08000000 --file fw.hex --verify
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

The generated YAML places the algorithm at `SRAM start + 0x20`; make sure the
SRAM range you provide is large enough for the algorithm (the command refuses
to emit a YAML whose algorithm does not fit).

## Output and exit codes

- Default output is human-readable. Add `--json` for machine-readable JSON that
  mirrors the MCP tool payloads. Logs always go to stderr.
- Exit codes: `0` success, `1` runtime error, `2` usage error, `3` aborted or
  destructive operation missing confirmation.

## Destructive operations

`flash erase`, `flash program` and destructive script commands (`erase`,
`loadbin`, `loadfile`, `flash write_image`, `flash erase_sector`) are gated:

- With a terminal, you are asked to confirm unless `--yes` is given.
- Without a terminal, `--yes` is required; otherwise the command is refused
  with exit code 3.
- In the `repl`, destructive mode is off by default; enable it interactively
  when prompted or start the REPL with `--yes`.

Flash erasing and programming can permanently damage a device. Double-check
the address and file before confirming.
