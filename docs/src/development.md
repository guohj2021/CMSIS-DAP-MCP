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

## Code style

- `cargo fmt --check` must pass; format with `cargo fmt` before committing.
- `cargo clippy --workspace --all-targets -- -D warnings` must pass with no
  warnings.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/):
  `type(scope): subject`, where type is one of `feat`, `fix`, `docs`,
  `refactor`, `test`, `chore`, `perf`. Reference the CHANGELOG style for
  examples.
- The repository must stay free of vendor-specific terms; run
  `scripts/check-no-vendor.ps1` (Windows PowerShell) before pushing. The
  CI enforces this check.

## Contributing

- Branch strategy: feature branches merge into `develop`, then `develop`
  merges into `main`. Pushing a `vX.Y.Z` tag triggers the release workflow.
- Pull requests must pass the full CI suite (fmt / clippy / test / build on
  Windows, Linux and macOS) and the vendor-content scan.
- Hardware-verified changes are preferred: when a feature touches probe or
  target behavior, validate it on a real CMSIS-DAP probe + Cortex-M board
  before opening the PR.
- Keep the English and Chinese documentation in sync: any user-visible
  change in `docs/src/` must be mirrored in `docs/zh/src/`.

## Testing strategy

- **Unit tests**: per-crate tests under `crates/*/tests/` cover backend
  behavior (mock and probe-rs), SVD parsing, hex encoding, register hints,
  security policy, session management and scripting.
- **Integration tests**: `crates/cmsis-dap-cli/tests/` exercise the CLI
  end to end (args, commands, live monitors, non-invasive dump);
  `crates/cmsis-dap-mcp/tests/` cover MCP handlers and feature flags.
- **Hardware verification**: before each release, run a full end-to-end
  session on a real CMSIS-DAP probe + Cortex-M target (list probes, connect,
  read/write memory, halt/resume, register access, flash program with
  verify, live watch / RTT / Event Recorder monitors, non-invasive dump).
  This is manual and not part of CI.

## Documentation maintenance

Before each release, run this checklist to keep documentation in sync with
code:

1. Compare `CHANGELOG.md` against the actual diff since the last release;
  add missing entries under the `## [vX.Y.Z] - unreleased` section.
2. Audit every README (`README.md`, `npm/README.md`, `npm-cli/README.md`)
  against the current feature set; update tool tables and configuration
  examples.
3. Verify `docs/src/SUMMARY.md` and `docs/zh/src/SUMMARY.md` reflect the
  current chapter list and the user/developer grouping.
4. Check `docs/src/tools.md` against the MCP tool implementations in
  `crates/cmsis-dap-mcp/src/mcp/`; add any new tool and its parameters.
5. Check `docs/src/architecture.md` module table against
  `crates/cmsis-dap-core/src/`; add any new module.
6. Sync the Chinese mirror in `docs/zh/src/` — structure, examples and
  command output must match the English version.
7. Build both books locally:
  ```bash
  mdbook build docs        # English
  mdbook build docs/zh     # Chinese
  ```
8. Run the vendor-content scan:
  ```powershell
  powershell -File scripts/check-no-vendor.ps1
  ```

If any step surfaces a discrepancy, fix it before tagging the release.

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
