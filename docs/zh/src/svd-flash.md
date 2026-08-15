# SVD 与 Flash

## SVD 文件

SVD 文件描述芯片的外设与寄存器。运行时提供你自己的文件：

```text
load_svd { "path": "/path/to/your-chip.svd" }
list_peripherals {}
read_peripheral { "peripheral": "GPIOA", "register": "ODR" }
write_peripheral { "peripheral": "GPIOA", "register": "ODR", "field": "ODR0", "value": 1 }
```

位域写入为读-改-写。本仓库绝不捆绑芯片专有数据。

## Flash 编程

Flash 工具需要带烧写算法的目标描述。从芯片的 CMSIS-Pack 生成 probe-rs 目标
YAML（或手写），并以此启动服务器：

```bash
cmsis-dap-mcp --target-yaml /path/to/your-target.yaml --allow-destructive
```

用 YAML 中定义的目标名连接，然后擦除与编程：

```text
connect { "protocol": "swd", "target": "YourChip" }
erase_flash { "address": 0x08000000, "size": 0x1000 }
program_flash { "address": 0x08000000, "data": [0x00, 0x11, ...], "verify": true }
```

`verify: true` 会在烧写后读回校验。`erase_flash` 只擦除与请求范围重叠的扇区；
传入完整 Flash 范围即整片擦除。

## 推荐的 Flash 流程（已实测）

1. 先读出当前固件并保留备份。
2. 只擦除要写入的扇区。
3. 以 `verify: true` 编程。
4. 读回并用 `verify_memory` 校验结果。
5. 若目标需保留原固件，则恢复备份。
