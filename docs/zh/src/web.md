# Web Debug（浏览器调试界面）

`cmsis-dap-cli web` 启动本地 Web Debug 服务，将同一调试引擎以浏览器
Debug Workspace 呈现：连接 CMSIS-DAP 探针、查看寄存器 / 内存 / 外设、
设置断点、实时观察变量、读取 RTT / EVR 输出、反汇编、查看调用栈与
局部变量（DWARF），并支持固件下载——无需 Keil / IAR / Eclipse / VS Code。

## 启动

```bash
cmsis-dap-cli web
```

默认绑定 `127.0.0.1:8080` 并提供单页应用。参数：

| 参数 | 说明 |
| --- | --- |
| `--host` | 绑定地址（默认 `127.0.0.1`；仅在有意图时使用 `0.0.0.0`） |
| `--port` | 绑定端口（默认 `8080`） |
| `--allow-destructive` | 启用 Flash 擦除/烧录与 Flash 软件断点 |
| `--flash-timeout` | Flash 操作超时秒数（默认 600） |

其余全局参数（`--target`、`--target-yaml`、`--flm`、`--probe-id`、`--svd`、
`--elf`、`--protocol`、`--speed-khz`、`--under-reset`、`--core-index`）会作为
Web 会话的默认值。

由 Keil FLM 定义的自定义芯片示例：

```bash
cmsis-dap-cli web --target-yaml target/demo.yaml --probe-id <序列号> --allow-destructive
```

## 工作区

- **Debug** — 可停靠面板：Target/Symbols、代码/反汇编、CPU 寄存器、
  Watch/Live Watch、内存、外设（SVD）、断点、调用栈、Locals、RTT、EVR、
  SWO、控制台。Quick Debug 与 Full Debug 是同一工作区的布局预设，
  布局保存在浏览器中。
- **Flash** — 拖入 BIN/HEX 文件，分析（地址范围）后执行
  擦除 / 烧录 / 校验 / 复位 / 运行，带实时进度。烧录为破坏性操作，
  必须显式确认。

## 亮点

- **符号 → Watch / 断点** — 上传 ELF/AXF（仅调试信息，不会烧录），
  搜索函数/变量，拖拽符号到 Watch，或按符号名设置断点。
- **Live Watch** — 目标运行中实时刷新变量值，每个条目可选刷新周期
  （50 ms … 1 s）。寄存器条目仅在暂停时读取。
- **外设浏览器** — 上传 SVD 文件，浏览寄存器并解码位域，且只周期刷新
  被监控的寄存器。
- **调用栈 + Locals** — 真实 DWARF CFI 解栈与 DWARF 变量求值
  （类型：基础/指针/typedef/结构体/数组/枚举）。无解栈信息或无法读取的
  值会如实显示"不可用"——绝不伪造。
- **表达式** — 对符号、寄存器（`$pc`、`$sp`）与内存（`*(0x20000000)`）
  求值类 C 表达式。
- **Fault** — 核心暂停时显示 CFSR/HFSR/DFSR/MMFAR/BFAR，任一置位时显示
  Fault 横幅。
- **命令面板** — `Ctrl+Shift+P` 执行常用命令。

## WebSocket 事件

UI 通过 `/ws` 接收增量事件：`target_state_changed`、
`live_watch_value_changed`、`peripheral_value_changed`、`rtt_output`、
`evr_event`、`flash_progress`、`flash_complete`、`probe_lost` 等。
`probe_lost` 会立即失效会话（USB 拔出后 UI 不会停留在"已连接"）。

## 安全

默认仅绑定 `127.0.0.1` 本地暴露。破坏性操作（Flash）由
`--allow-destructive` 门控并在 UI 中二次确认。
