# Troubleshooting

## Probe not listed

- **Windows**: CMSIS-DAP v2 probes need a WinUSB driver. Use
  [Zadig](https://zadig.akeo.ie/) to replace the driver with WinUSB if the
  probe does not appear. CMSIS-DAP v1 (HID) probes usually work without a
  driver.
- **Linux**: install a udev rule granting access to the USB device (see
  README), then replug the probe.
- **macOS**: usually works out of the box; check System Settings > Privacy &
  Security if the probe is blocked.

## Connect fails

- Check the wiring: SWDIO/SWCLK (and nRST when using `under_reset`).
- Lower the speed: `connect { "speed_khz": 100 }`.
- Try `under_reset: true` for locked targets.
- JTAG fails with `ConnectFailed` on targets that do not expose a JTAG TAP;
  use SWD instead.

## Register name errors

Names are case-insensitive and role-aware (`pc`, `sp`, `fp`, `lr`, `ra`,
`psr`, `xpsr`, `msp`, `psp`, `fpsr`, `r0`-`r15`). Other names must match the
architecture register file; use `list_core_registers` to see what is
available.

## Flash tools return `DestructiveDisabled`

Start the server with `--allow-destructive`.

## Flash algorithm fails to load

- The target YAML must define a RAM region large enough for the algorithm,
  the header and the stack.
- `load_address` must leave room for the 4-byte algorithm header, e.g.
  `0x20000020` for a RAM region starting at `0x20000000`.
- `pc_init`, `pc_uninit`, `pc_erase_sector`, `pc_program_page` and
  `pc_erase_all` in the YAML are **offsets** from the code start address.

## File formats and scripts

- `bin` files have no address information: always pass `address` (or the
  script `loadbin` address).
- `axf` files are ELF containers: use format `axf` or `auto`; they are parsed
  with the ELF loader.
- `hex` files are standard Intel HEX (type 00/04/01); invalid checksums or
  records return `FileError`.
- A valid ELF/AXF must contain loadable sections; an ELF with no sections
  fails with `FileError` ("no loadable segments").
- Scripts stop at the first failing command; check the per-command `status`
  and `output` in the result.

## AI client does not show the tools

- Restart the client after adding the server.
- For Codex, `codex mcp list` must show the server as enabled; the desktop
  app loads it when a new session starts.
- Verify the binary path in the client configuration is correct and
  executable.
