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

## AI 客户端不显示工具

- 添加服务器后重启客户端。
- Codex：`codex mcp list` 必须显示服务器已启用；桌面端在新会话启动时加载。
- 检查客户端配置中的二进制路径是否正确且可执行。
