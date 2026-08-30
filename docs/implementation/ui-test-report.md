# Web UI 功能测试报告（真机）

- 日期：2026-08-30
- 硬件：DemoMCU（Cortex-M0+，FLASH 0x08000000-0x08010000，SRAM 0x20000000-0x20002000）+ CMSIS-DAP CMSIS-DAP
- 环境：`cmsis-dap-cli web --port 18080 --allow-destructive --target-yaml target/demo.yaml`
- 工具：Playwright + Edge（headless），脚本位于 `target/ui_tests/`
- 测试方式：完全通过 UI 操作（点击/上传/拖拽）驱动，后端仅作状态校验

## 一、测试矩阵与结果

### A. 连接
| 用例 | 结果 |
|---|---|
| 初始未连接状态（连接可用、调试按钮禁用） | ✅ |
| 连接成功（出现断开按钮、目标信息） | ✅ |
| 断开成功（回到连接按钮） | ✅ |

### B. 执行控制
| 用例 | 结果 |
|---|---|
| 连接后目标状态同步 | ✅ |
| 暂停：运行可用/暂停禁用/单步可用 | ✅ |
| 单步执行 | ✅ |
| 运行：暂停可用/运行禁用 | ✅ |
| 复位并暂停 | ✅ |
| 复位（运行） | ✅ |

### C. 寄存器
| 用例 | 结果 |
|---|---|
| 暂停后寄存器列表（26 行，含 R0-R15/SP/PC/xPSR） | ✅ |
| HEX/DEC/BIN 格式切换 | ✅ |
| 按名称过滤 | ✅ |

### D. 内存
| 用例 | 结果 |
|---|---|
| 读取 SRAM（0x20000000，128 字节） | ✅ |
| u8/u16/u32 宽度切换 | ✅ |
| 单字节编辑写入并重读一致 | ✅ |

### E. 断点
| 用例 | 结果 |
|---|---|
| 添加硬件断点（列表显示） | ✅ |
| 清除全部断点 | ✅（修复后） |

### F. 符号（ELF/AXF）
| 用例 | 结果 |
|---|---|
| 上传 AXF 后符号列表渲染（104 行） | ✅ |
| 搜索/筛选（函数/变量） | ✅ |
| ＋Watch 按钮加入 Watch | ✅ |
| 拖拽符号到 Watch | ✅ |

### G. Watch
| 用例 | 结果 |
|---|---|
| Watch 项显示与周期切换（50ms-5s） | ✅ |
| 删除 Watch 项 | ✅ |

### H. SVD / 外设
| 用例 | 结果 |
|---|---|
| 上传 SVD，外设列表（35 个） | ✅ |
| 打开外设显示寄存器/地址/访问属性（GPIOA 11 个） | ✅ |
| 读寄存器：值 + 位域仅在该行显示（1 处） | ✅ |
| 外设监控添加 + 周期选择 | ✅ |
| 监控周期修改（200→1000ms 生效） | ✅ |
| 监控值周期刷新（WS） | ✅ |
| 停止监控 | ✅ |

### I. Flash（BIN/HEX）
| 用例 | 结果 |
|---|---|
| 上传 HEX 分析（类型/大小/范围/段数） | ✅ |
| 擦除 | ✅（需芯片 target） |
| 烧录 + 校验 | ✅ |
| 烧录后复位并暂停 | ✅ |
| 固件校验（读回 0x08000000 向量表） | ✅ |

### J. RTT / EVR
| 用例 | 结果 |
|---|---|
| RTT 启动 | ⚠ 固件无 RTT 控制块（演示固件未初始化 RTT），UI 正确报错 |
| EVR 面板 | ⚠ 固件 Event Recorder 协议不受 probe-rs 支持 |

### K. 布局 / 菜单 / 配置
| 用例 | 结果 |
|---|---|
| 布局 Quick(4)/Full 切换 | ✅ |
| 窗口菜单（15 个窗口显隐） | ✅ |
| 配置保存/列出/加载/删除 | ✅ |
| 刷新后布局持久化 | ✅ |

**合计：核心套件 27/27 通过；外设/Flash 全通过；RTT/EVR 为固件能力限制（非 Web 缺陷）。**

## 二、发现并修复的 Bug

### BUG-1：断点"清除"按钮失效（405 → 500）
- 现象：断点面板点"清除"无反应，浏览器控制台出现 405/500。
- 根因：前端调用 `DELETE /api/breakpoints`，后端该路由只注册了 GET/POST（405）；补上 DELETE 后用了带 `Path<u64>` 的 handler（无路径参数 → 500）。
- 修复：`api.rs` 新增无参 `breakpoint_clear_all` handler 并注册到 `DELETE /api/breakpoints`（执行 clear-all）。
- 验证：`DELETE /api/breakpoints` → `{"cleared":true}` 200；UI 清除正常。

### BUG-2：会话失效/断开后状态轮询持续 409
- 现象：断开/探针丢失/服务重启后，前端仍每 500ms 轮询 `/api/status`，不断返回 409，UI 停在"已连接"。
- 根因：轮询器只在成功时更新状态，收到 `not_connected` 不处理，连接状态不回落。
- 修复：`App.tsx` 轮询器在收到 `error.code == "not_connected"` 时把状态置为 `disconnected`，停止轮询并同步 UI。
- 验证：断开后无 409 风暴，UI 回到"未连接"。

### BUG-3（配置要求）：Flash 需要芯片 target
- 现象：未带 `--target-yaml` 启动时，擦除报 `target has no flash memory definition`，烧录被拒。
- 根因：泛型目标无 Flash 算法；DemoMCU 需要芯片 YAML（`chip generate --flm` 生成）。
- 处理：以 `--target-yaml target/demo.yaml` 启动后 Flash 全流程通过；在 `web` 子命令帮助中补充说明。
- 注意：这是运行配置要求，不是代码缺陷；错误信息已明确。

### 固件能力限制（非缺陷）
- RTT：演示固件未初始化 SEGGER RTT 控制块 → 400（提示明确）。
- EVR：DemoMCU Event Recorder 协议与 probe-rs 不兼容 → 400。

## 三、回归
- `cargo test --workspace`：34/34 套件通过
- `cargo clippy --workspace --all-targets`：无告警
- `cargo fmt --check`：通过
- 原 CLI/MCP 行为不变（仅新增路由/处理，无公共 API 删除/改名）
