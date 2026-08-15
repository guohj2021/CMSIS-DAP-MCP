# SVD and Flash

## SVD files

SVD files describe a chip's peripherals and registers. Provide your own file
at runtime:

```text
load_svd { "path": "/path/to/your-chip.svd" }
list_peripherals {}
read_peripheral { "peripheral": "GPIOA", "register": "ODR" }
write_peripheral { "peripheral": "GPIOA", "register": "ODR", "field": "ODR0", "value": 1 }
```

Field writes are read-modify-write. This repository never bundles
chip-specific data.

## Flash programming

Flash tools need a target description with a flash algorithm. Generate a
probe-rs target YAML from your chip's CMSIS-Pack (or write one by hand) and
start the server with it:

```bash
cmsis-dap-mcp --target-yaml /path/to/your-target.yaml --allow-destructive
```

Connect with the target name defined in the YAML, then erase and program:

```text
connect { "protocol": "swd", "target": "YourChip" }
erase_flash { "address": 0x08000000, "size": 0x1000 }
program_flash { "address": 0x08000000, "data": [0x00, 0x11, ...], "verify": true }
```

`verify: true` reads the data back after programming. `erase_flash` erases
only the sectors overlapping the requested range; pass the full flash range
to erase the whole chip.

## Recommended flash workflow (verified)

1. Read out the current firmware first and keep it as a backup.
2. Erase only the sectors you intend to write.
3. Program with `verify: true`.
4. Read back and `verify_memory` the result.
5. Restore the backup if the target must keep its original firmware.
