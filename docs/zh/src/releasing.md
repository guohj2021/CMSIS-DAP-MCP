# 发布流程

本项目遵循轻量级 Git Flow 约定：

- `main` — 稳定、已发布的代码。GitHub Pages 文档站点从此分支构建。
- `develop` — 默认集成分支（`origin/HEAD`）。所有工作都合入这里。
- 功能 / 修复 / 发布分支 — 短生命周期；通过 Pull Request 合入 `develop`，
  合并后**及时删除**。仓库已开启合并后自动删除源分支
  （`delete_branch_on_merge`）。

## 分支规则

- 始终从最新的 `develop` 拉分支，并开 PR 合回 `develop`。
- 合并前 CI（构建 + 测试）必须通过。
- PR 合并后源分支会被自动删除——不要遗留陈旧分支。该合并的合并，合并完
  的及时删除。
- `main` 只能通过从 `develop` 同步（发布同步 PR）更新，绝不直接把功能
  提交到 `main`。

## 发布流程

1. **同步检查。** 打标签前 `main` 和 `develop` 必须指向同一提交。运行
   `scripts/check-release-sync.ps1`，或依赖 `Release` 工作流——两者不一致时
   工作流会拒绝执行。
2. **在 `develop` 上更新版本与变更日志：** 升级 `Cargo.toml` workspace 版本、
   `web/package.json`、`npm/package.json`、`npm-cli/package.json` 及平台子包；
   在 `CHANGELOG.md` 增加条目；更新固定版本文档与 README 的 Release 徽章
   （`?branch=vX.Y.Z`）。
3. **用 PR 将 `develop` 同步到 `main`**（合并后源分支自动删除）。
4. **在 `develop` 上打 `vX.Y.Z` 标签并推送。** `Release` 工作流会：
   - 首先校验 `main == develop`（不一致则失败），
   - 构建 Web UI 与全平台发布二进制，
   - 创建带二进制的 GitHub Release，
   - 发布 `cmsis-dap-mcp` 和 `cmsis-dap-cli` npm meta 包及 16 个平台包。

## 检查清单

运行发布前检查：

```powershell
scripts/check-release-sync.ps1
```

脚本会拉取远端、校验工作区干净、确认 `main == develop`，并列出应当删除的
已合并远端分支。
