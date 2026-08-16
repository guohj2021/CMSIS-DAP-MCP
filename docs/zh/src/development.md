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
