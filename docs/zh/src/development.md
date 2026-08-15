# 开发与发布

## 构建与测试

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

## 文档

```bash
mdbook build docs     # 英文
mdbook build docs/zh  # 中文
```

## 发布流程

仓库遵循 GitFlow：特性分支合并到 `develop`，再合并 `develop` 到 `main`。
推送 `vX.Y.Z` tag 会触发发布工作流：构建三个平台二进制、发布 npm 元包与
平台包、上传 GitHub Release 资产，并重建 GitHub Pages 文档。

发布前请在真实硬件上跑完整验证套件，并执行厂商内容扫描：

```powershell
powershell -File scripts/check-no-vendor.ps1
```
