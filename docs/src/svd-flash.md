# SVD and Flash

## SVD files

SVD files describe a chip's peripherals and registers. Provide your own file at runtime:

```text
load_svd { "path": "/path/to/your-chip.svd" }
list_peripherals {}
read_peripheral { "peripheral": "GPIOA", "register": "ODR" }
write_peripheral { "peripheral": "GPIOA", "register": "ODR", "field": "ODR0", "value": 1 }
```

Field writes are read-modify-write. This repository never bundles chip-specific data.

## Flash programming

Flash tools need a target description with a CMSIS-Pack flash algorithm. Connect with a target name probe-rs can resolve, then:

```text
program_flash { "address": 0x08000000, "data": [0x00, 0x11, ...] }
```

Requires `--allow-destructive`.