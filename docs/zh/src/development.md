# 开发与发布

## 构建与测试

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
```

这会构建两个二进制：`target/release/cmsis-dap-mcp`（MCP 服务器）与
`target/release/cmsis-dap-cli`（命令行工具）。

## 代码规范

- `cargo fmt --check` 必须通过；提交前用 `cargo fmt` 格式化。
- `cargo clippy --workspace --all-targets -- -D warnings` 必须通过且无警告。
- 提交信息遵循 [Conventional Commits](https://www.conventionalcommits.org/)：
  `type(scope): subject`，type 为 `feat`、`fix`、`docs`、`refactor`、
  `test`、`chore`、`perf` 之一。参考 CHANGELOG 风格示例。
- 仓库不得出现厂商专有词；推送前运行 `scripts/check-no-vendor.ps1`
  （Windows PowerShell）。CI 会强制执行此检查。

## 贡献指南

- 分支策略：特性分支合并到 `develop`，再由 `develop` 合并到 `main`。
  推送 `vX.Y.Z` tag 会触发发布工作流。
- PR 必须通过完整 CI 套件（Windows、Linux、macOS 三平台的 fmt / clippy /
  test / build）与厂商内容扫描。
- 涉及探针或目标行为的变更建议在真实硬件上验证：在真实 CMSIS-DAP 探针 +
  Cortex-M 板子上跑通后再开 PR。
- 中英文文档必须同步：`docs/src/` 的任何用户可见变更都要镜像到
  `docs/zh/src/`。

## 测试策略

- **单元测试**：各 crate 的 `crates/*/tests/` 覆盖后端（mock 与 probe-rs）、
  SVD 解析、hex 编码、寄存器提示、安全策略、会话管理与脚本。
- **集成测试**：`crates/cmsis-dap-cli/tests/` 端到端测试 CLI（参数、命令、
  实时监控、非侵入 dump）；`crates/cmsis-dap-mcp/tests/` 覆盖 MCP 处理器
  与功能开关。
- **硬件验证**：每次发布前，在真实 CMSIS-DAP 探针 + Cortex-M 目标上跑
  完整端到端会话（枚举探针、连接、读写内存、halt/resume、寄存器访问、
  带校验的 Flash 烧录、实时 watch / RTT / Event Recorder 监控、非侵入
  dump）。此步骤为手工验证，不在 CI 中执行。

## 文档维护

每次发布前，按以下清单保持文档与代码同步：

1. 对照 `CHANGELOG.md` 与上次发布以来的实际 diff，在
   `## [vX.Y.Z] - unreleased` 段补充遗漏条目。
2. 审计所有 README（`README.md`、`npm/README.md`、`npm-cli/README.md`），
   按当前功能集更新工具表与配置示例。
3. 校对 `docs/src/SUMMARY.md` 与 `docs/zh/src/SUMMARY.md`，确保章节列表
   与用户/开发者分组反映当前状态。
4. 对照 `docs/src/tools.md` 与 `crates/cmsis-dap-mcp/src/mcp/` 的 MCP 工具
   实现，补充新工具及其参数。
5. 对照 `docs/src/architecture.md` 模块表与
   `crates/cmsis-dap-core/src/`，补充新模块。
6. 同步 `docs/zh/src/` 中文镜像——结构、示例与命令输出必须与英文版一致。
7. 本地构建两份书：
   ```bash
   mdbook build docs        # 英文
   mdbook build docs/zh     # 中文
   ```
8. 运行厂商内容扫描：
   ```powershell
   powershell -File scripts/check-no-vendor.ps1
   ```

任一步骤发现差异，必须在打 tag 发布前修正。

## 文档

```bash
mdbook build docs     # 英文
mdbook build docs/zh  # 中文
```

## 发布流程

仓库遵循 GitFlow：特性分支合并到 `develop`，再合并 `develop` 到 `main`。
推送 `vX.Y.Z` tag 会触发发布工作流：构建三个平台二进制、发布 npm 元包与
平台包（`cmsis-dap-mcp` 与 `cmsis-dap-cli` 两套）、上传 GitHub Release
资产，并重建 GitHub Pages 文档。

发布前请在真实硬件上跑完整验证套件，并执行厂商内容扫描：

```powershell
powershell -File scripts/check-no-vendor.ps1
```
