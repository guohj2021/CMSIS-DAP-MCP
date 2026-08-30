# Web UI 功能测试（Playwright + Edge）

通过真实浏览器操作 Web Debug 界面进行功能回归测试。依赖：
- 本地 `cmsis-dap-cli web` 服务（默认 http://127.0.0.1:18080/）
- 真实调试器 + MCU（测试用 DemoMCU + CMSIS-DAP CMSIS-DAP）
- Python 3 + playwright（`pip install playwright`，浏览器用系统 Edge）

Flash 测试需要服务以芯片 target 启动（否则无 Flash 算法）：
```
cmsis-dap-cli web --port 18080 --allow-destructive --target-yaml target/demo.yaml
```

## 配置（环境变量）
| 变量 | 说明 | 默认 |
|---|---|---|
| `UI_URL` | 服务地址 | http://127.0.0.1:18080/ |
| `UI_AXF` | 符号测试用 ELF/AXF | 本机 demo.axf |
| `UI_SVD` | 外设测试用 SVD | 本机 DemoMCU.svd |
| `UI_HEX` | Flash 测试用固件 | 本机 demo.hex |
| `UI_EDGE` | msedge.exe 路径 | 自动探测 |

## 运行
```
python tests/ui/test_core.py    # 连接/执行/寄存器/内存/断点/符号/Watch/布局菜单配置
python tests/ui/test_extra.py   # SVD/外设监控 + RTT/EVR（外设部分）
python tests/ui/test_flash.py   # Flash 擦除/烧录/校验（破坏性，会覆盖 MCU 固件）
```
报告输出到 `reports/ui/*.json`，汇总与 Bug 记录见 `docs/implementation/ui-test-report.md`。
