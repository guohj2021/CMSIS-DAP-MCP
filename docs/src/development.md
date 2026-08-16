# Development

## Build and test

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
```

This builds both binaries: `target/release/cmsis-dap-mcp` (MCP server) and
`target/release/cmsis-dap-cli` (CLI).

## Documentation

```bash
mdbook build docs     # English
mdbook build docs/zh  # Chinese
```

## Release process

The repository follows GitFlow: feature branches merge into `develop`, then
`develop` into `main`. Pushing a `vX.Y.Z` tag triggers the release workflow,
which builds the three platform binaries for both tools, publishes the
`cmsis-dap-mcp` and `cmsis-dap-cli` npm meta packages plus their platform
packages, uploads GitHub Release assets, and rebuilds the GitHub Pages
documentation.

Before releasing, run the full verification suite on real hardware and the
vendor-content scan:

```powershell
powershell -File scripts/check-no-vendor.ps1
```
