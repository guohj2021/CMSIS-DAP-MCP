# 故障排查

## 探针未列出

- **Windows**：CMSIS-DAP v2 探针需要 WinUSB 驱动。如果探针不出现，请用
  [Zadig](https://zadig.akeo.ie/) 把驱动替换为 WinUSB。CMSIS-DAP v1（HID）
  探针通常无需驱动。
- **Linux**：安装授予 USB 设备访问权限的 udev 规则（见 README），然后重新
  插拔探针。
- **macOS**：通常开箱即用；若探针被拦截，检查系统设置 > 隐私与安全性。

## 连接失败

- 检查接线：SWDIO/SWCLK（使用 `under_reset` 时还需 nRST）。
- 降低速度：`connect { "speed_khz": 100 }`。
- 锁定目标可尝试 `under_reset: true`。
- 目标未引出 JTAG TAP 时，JTAG 会返回 `ConnectFailed`，请改用 SWD。

## 寄存器名错误

名称大小写不敏感且按角色解析（`pc`、`sp`、`fp`、`lr`、`ra`、`psr`、`xpsr`、
`msp`、`psp`、`fpsr`、`r0`-`r15`）。其他名称必须匹配架构寄存器表；可用
`list_core_registers` 查看可用项。

## Flash 工具返回 `DestructiveDisabled`

以 `--allow-destructive` 启动服务器。

## Flash 算法加载失败

- 目标 YAML 必须定义足够大的 RAM 区域以容纳算法、header 与栈。
- `load_address` 必须为 4 字节算法 header 留出空间，例如 RAM 从
  `0x20000000` 开始时用 `0x20000020`。
- YAML 中的 `pc_init`、`pc_uninit`、`pc_erase_sector`、`pc_program_page`、
  `pc_erase_all` 是**相对代码起始地址的偏移**。

## 文件格式与脚本

- `bin` 文件没有地址信息：必须显式给 `address`（或脚本 `loadbin` 的地址）。
- `axf` 是 ELF 容器：用 `axf` 或 `auto` 格式，走 ELF 解析。
- `hex` 是标准 Intel HEX（type 00/04/01）；校验和或记录非法时返回
  `FileError`。
- 有效的 ELF/AXF 必须包含可加载节；无节的 ELF 会以 `FileError`
  （"no loadable segments"）失败。
- 脚本遇到第一条失败命令即停止；请查看返回结果中每条命令的 `status` 与
  `output`。

## AI 客户端不显示工具

- 添加服务器后重启客户端。
- Codex：`codex mcp list` 必须显示服务器已启用；桌面端在新会话启动时加载。
- 检查客户端配置中的二进制路径是否正确且可执行。

## RTT / Event Recorder

- **`RTT attach failed: control block not found`** —— 固件必须初始化
  SEGGER RTT（`SEGGER_RTT_Init()`），且主机要在初始化之后、核心进入
  `main` 运行后再附着（不能在停机时附着）。传入 `--elf`（`_SEGGER_RTT`
  符号）或 `--address`，并在 `repl` 里 `reset run` 后运行监控，确保核心
  真正在执行。
- **`evr` 需要地址** —— 固件必须包含 CMSIS-View Event Recorder 组件
  （符号 `EventRecorderInfo`）。传入 `--elf` 或 `--address`，并在附着前用
  `EventRecorderInitialize` 完成初始化。
- **一次性命令读到的是旧值** —— 每次一次性调用都会新建会话，probe-rs
  附着时核心处于停机。请在 `repl` 中 `connect` + `reset run`，再执行
  `watch run` / `rtt monitor` / `evr monitor`。
- **EVR 秒数看起来不对** —— 秒数由固件 `ts_freq`
  （`EVENT_TIMESTAMP_FREQ`）换算；请把它设为实际时间戳时钟（例如
  `SystemCoreClock`）。tick 本身始终单调递增。
