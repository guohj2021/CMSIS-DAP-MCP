# Releasing

This project follows a lightweight Git Flow convention:

- `main` — stable, released code. The GitHub Pages docs site is built from
  here, and the release tag-triggered workflow publishes from it.
- `develop` — the default integration branch (`origin/HEAD`). All work lands
  here.
- Feature / fix / release branches — short-lived; merged into `develop` via a
  pull request, then **deleted promptly**. The repository auto-deletes head
  branches on merge (`delete_branch_on_merge`).

## Branch rules

- Always branch from the latest `develop` and open a PR back to `develop`.
- CI (build + test) must pass before merging.
- After a PR merges, its head branch is deleted automatically — do not leave
  stale branches around. Merge what is ready, delete what is merged.
- `main` only ever moves by syncing from `develop` (a release sync PR). Never
  commit feature work directly to `main`.

## Release process

1. **Sync check.** `main` and `develop` must point at the same commit before
   tagging. Run `scripts/check-release-sync.ps1`, or rely on the `Release`
   workflow, which refuses to run when they differ.
2. **Version bump + changelog** on `develop`: bump the `Cargo.toml` workspace
   version, `web/package.json`, `npm/package.json`, `npm-cli/package.json` and
   the platform sub-packages; add a `CHANGELOG.md` entry; update
   pinned-version docs and the README Release badge (`?branch=vX.Y.Z`).
3. **Sync `develop` → `main`** with a PR (merge, then the head branch is
   auto-deleted).
4. **Tag** `vX.Y.Z` on `develop` and push it. The `Release` workflow:
   - first verifies `main == develop` (fails otherwise),
   - builds the web UI and release binaries for every platform,
   - creates the GitHub Release with the binaries,
   - publishes the `cmsis-dap-mcp` and `cmsis-dap-cli` npm meta packages plus
     the 16 platform packages.

## Checklist

Run the pre-release check:

```powershell
scripts/check-release-sync.ps1
```

It fetches, verifies a clean tree, confirms `main == develop`, and lists any
merged remote branches that should be deleted.
