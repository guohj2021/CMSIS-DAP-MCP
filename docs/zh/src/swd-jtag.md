# SWD 与 JTAG

两种协议都支持。在 `connect` 时选择，或用服务器启动参数 `--protocol` 设置
默认值。

```text
connect { "protocol": "swd" }    # 默认
connect { "protocol": "jtag" }
```

`list_probes` 会报告已连接探针支持的协议。大多数 CMSIS-DAP 探针两种都支持。

## 如何选择

- **SWD** 是默认值，任何带调试端口的 Cortex-M 都可用，只需两根线（SWDIO、
  SWCLK）加复位。
- **JTAG** 需要目标引出 JTAG TAP 与四/五根 JTAG 引脚。很多小型
  Cortex-M0/M0+ 器件没有引出 JTAG。

服务器已在硬件上通过 SWD 验证。如果目标不支持 JTAG，`connect` 会返回
`ConnectFailed` 协议错误；对支持 JTAG 的目标，工具集完整支持 JTAG。

## 速度

在 `connect` 中传 `speed_khz`，或在启动时设置 `--speed-khz`。探针会选择不
高于请求的最高可用速度。

## 复位下连接

对于锁定或无响应的目标，可在连接期间保持复位线：

```text
connect { "protocol": "swd", "under_reset": true }
```

这要求探针的复位引脚已连接到目标的复位。
