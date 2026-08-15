# SWD and JTAG

Both protocols are supported. Select one at `connect` time, or set the
default with the `--protocol` server option.

```text
connect { "protocol": "swd" }    # default
connect { "protocol": "jtag" }
```

`list_probes` reports which protocols the connected probe supports. Most
CMSIS-DAP probes support both.

## Which one to use

- **SWD** is the default and works on any Cortex-M with a debug port. It uses
  two wires (SWDIO, SWCLK) plus reset.
- **JTAG** requires the target to expose a JTAG TAP and the four/five JTAG
  pins. Many small Cortex-M0/M0+ devices do not bring out JTAG.

The server was verified on hardware over SWD. If a target does not support
JTAG, `connect` returns `ConnectFailed` with a protocol error; the toolset
still fully supports JTAG for targets that expose it.

## Speed

Pass `speed_khz` in `connect`, or set `--speed-khz` at startup. The probe
selects the highest supported speed at or below the request.

## Connect under reset

For locked or non-responsive targets, hold the reset line during attach:

```text
connect { "protocol": "swd", "under_reset": true }
```

This requires the probe's reset pin to be wired to the target's reset.
