# Feature Roadmap: v0.6.0 & v0.7.0

> **For agentic workers:** Use superpowers:subagent-driven-development or
> superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close the feature gap with ST-Link (CubeProgrammer) and J-Link
(Commander/RTT Viewer) across two releases, focusing on the three highest-
value missing capabilities: SWO/SWV trace, option bytes, and flash software
breakpoints.

**Architecture impact:** All features extend the existing three-crate
workspace (`cmsis-dap-core` / `cmsis-dap-mcp` / `cmsis-dap-cli`). No new
crates needed. New backend trait methods, MCP tools, and CLI subcommands.

**Tech stack unchanged:** Rust stable, probe-rs 0.32.0, rmcp, tokio.

---

## Competitive Gap Summary

| Feature | CMSIS-DAP MCP | ST-Link | J-Link | Priority |
| --- | --- | --- | --- | --- |
| SWO/SWV Trace | missing | SWV real-time | SWO support | P0 |
| Option Bytes | missing | full RDP/WDG/Boot | encapsulated | P0 |
| Flash SW Breakpoints | missing | no | Unlimited Flash BP | P0 |
| Multi-core debug | missing | dual-core | multi-core | P1 |
| Memory-mapped read | missing | AHB-AP fast | fast read | P1 |
| GPIO control | missing | no | GPIO toggle | P2 |
| ITM decode | missing | SWV decode | ITM support | P2 |
| Web interface | missing | no | J-Link Web Server | P3 |

probe-rs 0.32.0 provides: `SwoConfig`, `SwoMode`, `SwoAccess` trait,
`SwoReader`, `setup_tracing()`, `disable_swv()`, `swo_reader()`.
probe-rs does NOT provide: option bytes, GPIO pin control. Those require
raw DAP register manipulation or CMSIS-DAP protocol extensions.

---

## v0.6.0 — SWO Trace + Option Bytes

### Task 1: SWO/SWV Trace (Backend)

**Files:** `crates/cmsis-dap-core/src/backend/mod.rs`,
`crates/cmsis-dap-core/src/backend/probe_rs.rs`

1. Add to `Backend` trait:
   ```rust
   fn start_swo(&mut self, baud: u32, tpiu_clk: u32) -> Result<(), McpError>;
   fn stop_swo(&mut self) -> Result<(), McpError>;
   fn read_swo_data(&mut self) -> Result<Vec<u8>, McpError>;
   ```
2. Implement in `ProbeRsBackend` using `session.setup_tracing(
   TraceSink::Swo(SwoConfig::new(tpiu_clk).set_baud(baud)))` and
   `session.swo_reader()` / `session.disable_swv()`.
3. Add `UnsupportedFeature` defaults in `MockBackend`.
4. Unit test: mock backend returns empty data; probe-rs backend compiles.

### Task 2: SWO Trace (MCP Tools)

**Files:** `crates/cmsis-dap-mcp/src/mcp/tools_swo.rs` (new),
`crates/cmsis-dap-mcp/src/mcp/mod.rs`

1. Create `tools_swo.rs` with three tool parameter structs:
   - `StartSwoParams { baud: u32, tpiu_clk: u32 }`
   - `StopSwoParams {}`
   - `ReadSwoParams { max_bytes: Option<u32> }`
2. Register in `#[tool_router]` impl block on `CmsisDapMcp`:
   - `start_swo` (write level) — configure and enable SWO
   - `stop_swo` (write level) — disable SWO
   - `read_swo` (read level) — read available SWO bytes, return as
     base64 or raw JSON array
3. Update `SERVER_INSTRUCTIONS` to mention SWO tools.
4. Add to `tools.md` and `cli.md` docs (EN + ZH).

### Task 3: SWO Trace (CLI)

**Files:** `crates/cmsis-dap-cli/src/cmd/mod.rs`,
`crates/cmsis-dap-cli/src/cmd/swo.rs` (new)

1. Add `Swo` subcommand to `Command` enum with sub-actions:
   - `start --baud 2000000 --tpiu-clock 8000000`
   - `stop`
   - `monitor --count 0 --interval-ms 100 [--log-dir DIR]`
2. `swo monitor` polls `read_swo_data()` in a loop, prints timestamped
   raw bytes to stdout (hex dump or ASCII), exports to log file.
3. `--json` mode outputs NDJSON with `host_ts` field (same pattern as
   `watch`/`rtt`/`evr`).
4. Integration test: connect, start_swo, read, stop (requires hardware).

### Task 4: Option Bytes (Backend)

**Files:** `crates/cmsis-dap-core/src/backend/mod.rs`,
`crates/cmsis-dap-core/src/backend/probe_rs.rs`

Note: probe-rs 0.32.0 does not have native option byte support. Implement
via raw DAP register access (`read_dap`/`write_dap`) targeting the
manufacturer's option byte address. This is chip-family specific but the
common pattern (STM32: FLASH_OPTCR at 0x40023C14, read via AP) is stable.

1. Add to `Backend` trait:
   ```rust
   fn read_option_bytes(&mut self) -> Result<Vec<OptionByte>, McpError>;
   fn write_option_bytes(&mut self, bytes: &[OptionByte]) -> Result<(), McpError>;
   ```
2. Define `OptionByte` struct: `{ name: String, address: u32, value: u32,
   description: Option<String> }`.
3. Implement via `read_dap`/`write_dap` for common STM32 option byte
   layout. Document that this is best-effort and chip-family specific.
4. Destructive level: `write_option_bytes` is destructive (can lock the
   device). `read_option_bytes` is read-only.

### Task 5: Option Bytes (MCP + CLI)

**Files:** `crates/cmsis-dap-mcp/src/mcp/tools_option.rs` (new),
`crates/cmsis-dap-cli/src/cmd/option.rs` (new)

1. MCP tools: `read_option_bytes` (read), `write_option_bytes`
   (destructive).
2. CLI commands: `option read`, `option write --name RDP --value 0`.
3. Documentation update (EN + ZH).

### Task 6: Documentation + Tests

1. Update `docs/src/tools.md` + `docs/zh/src/tools.md` with SWO and
   option byte sections.
2. Update `docs/src/cli.md` + `docs/zh/src/cli.md` with `swo` and
   `option` command references.
3. Update README feature tables (EN + ZH).
4. Update CHANGELOG.md with v0.6.0 entries.
5. `cargo test --workspace` passes.
6. `scripts/check-no-vendor.ps1` passes.
7. `mdbook build docs && mdbook build docs/zh` passes.

---

## v0.7.0 — Flash SW Breakpoints + Multi-core Prep

### Task 7: Flash Software Breakpoints

**Files:** `crates/cmsis-dap-core/src/backend/mod.rs`,
`crates/cmsis-dap-core/src/backend/probe_rs.rs`,
`crates/cmsis-dap-core/src/backend/flash_bp.rs` (new)

1. New module `flash_bp` implementing:
   - `FlashBpManager` struct tracking: active flash BPs (address ->
     original instruction), flash algorithm state.
   - `insert_flash_bp(backend, address)` — read original instruction,
     replace with `BKPT` (0xBE00 for Thumb), store original.
   - `remove_flash_bp(backend, address)` — restore original instruction.
   - `list_flash_bps()` — list active flash BPs.
   - `remove_all_flash_bps()` — restore all and clear.
2. Add to `Backend` trait:
   ```rust
   fn set_flash_breakpoint(&mut self, address: u64) -> Result<(), McpError>;
   fn clear_flash_breakpoints(&mut self) -> Result<(), McpError>;
   fn list_flash_breakpoints(&mut self) -> Result<Vec<u64>, McpError>;
   ```
3. Default impl returns `UnsupportedFeature`.
4. Important: flash BPs require `--allow-destructive` because they
   modify flash contents. Gate behind destructive policy.
5. Unit tests: mock backend tracks flash BP state.

### Task 8: Flash BP (MCP + CLI)

1. MCP tools: `set_flash_breakpoint` (destructive),
   `clear_flash_breakpoints` (destructive),
   `list_flash_breakpoints` (read).
2. CLI: `bp set-flash ADDR`, `bp clear-flash`, `bp list-flash`.
3. Integrate with existing `bp` command — `bp set` tries HW first, if
   `UnsupportedFeature` or slot full, suggest `bp set-flash`.

### Task 9: Multi-core Preparation

Note: Full multi-core is complex and deferred. v0.7.0 adds infrastructure
only.

1. Extend `ConnectOptions` with `core_index: Option<usize>` (default 0).
2. Extend `TargetInfo` with `cores: Vec<CoreInfo>` where `CoreInfo` has
   `index`, `core_type`, `name`.
3. MCP `connect` tool accepts optional `core` parameter.
4. CLI accepts `--core N` global parameter.
5. All existing operations default to core 0.

### Task 10: Documentation + Release Prep

1. Update all docs for new features.
2. Update CHANGELOG.md with v0.7.0 entries.
3. Bump workspace version to 0.7.0 in `Cargo.toml`.
4. Sync npm package.json versions.
5. Full test suite passes.

---

## Implementation Order

```text
v0.6.0 (current sprint):
  Task 1 -> Task 2 -> Task 3  (SWO end-to-end)
  Task 4 -> Task 5            (Option Bytes end-to-end)
  Task 6                     (docs + verify)
  PR -> develop -> main -> release v0.6.0

v0.7.0 (next sprint):
  Task 7 -> Task 8            (Flash SW Breakpoints)
  Task 9                     (Multi-core prep)
  Task 10                    (docs + release)
  PR -> develop -> main -> release v0.7.0
```

## Assumptions

- STM32 option byte layout is used as the reference implementation;
  other chip families can be added later.
- Flash SW breakpoints are limited to Thumb-2 BKPT instruction
  (0xBE00). ARM mode support deferred.
- Multi-core support is infrastructure-only in v0.7.0; full dual-core
  debugging is a future goal.
- SWO baud rate and TPIU clock must be provided by the user (auto-
  detection from target info is a future enhancement).
- All new features follow the existing three-tier security model.
