# CMSIS-DAP MCP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 交付一个通用、无厂商绑定、用户零依赖的 CMSIS-DAP MCP 服务器，让 AI 客户端能枚举探针、连接 Cortex-M、读写内存与外设、控制核心执行，并可选执行 Flash 编程。

**Architecture:** Rust 单进程 stdio MCP 服务器。MCP 工具层（rmcp）-> 安全策略层 -> 会话管理 -> 后端接口层（probe-rs 实现 + 测试用 MockBackend）。SVD 与 Flash 算法作为运行时用户资源，不进入仓库。

**Tech Stack:** Rust（stable 1.97.1，写入 `rust-toolchain.toml`）、rmcp（官方 Rust MCP SDK）、probe-rs、tokio、serde/serde_json/schemars、clap、thiserror、tracing/tracing-subscriber、mdBook（文档）。

## Global Constraints

- 仅 stdio 传输；日志只能写 stderr 或日志文件，绝不写 stdout。
- 破坏性工具（erase_flash、program_flash）默认禁用，仅 `--allow-destructive` 后可用。
- 工具清单、名称、安全等级必须与设计规格第 6 节完全一致。
- 每个 MCP 工具必须声明 annotations：readOnlyHint、destructiveHint、idempotentHint。
- 仓库（含 docs、README、CI、npm 包）不得包含任何厂商专有内容；`scripts/check-no-vendor.ps1`（扫描脚本本身除外）必须零命中。
- 本地实机验证资料（厂商 SDK、SVD、FLM、探针序列号）保持在仓库外；验证命令只记录在本地会话笔记，不提交。
- 依赖版本通过 `Cargo.lock` 锁定；发布前核对许可证（Apache-2.0/MIT 双许可）。
- SemVer 版本管理；GitFlow：feature -> develop -> main；tag `vX.Y.Z` 触发 Release。

---

## File Structure

```text
rust-toolchain.toml
Cargo.toml / Cargo.lock
src/main.rs                  CLI 入口：参数解析、日志、启动 rmcp stdio
src/cli.rs                   AppConfig 解析
src/error.rs                 ErrorCode / McpError
src/security.rs              SecurityLevel / SecurityPolicy
src/session.rs               SessionManager
src/backend/mod.rs           Backend trait、ProbeInfo、ConnectOptions、TargetInfo、AccessWidth、CoreRegister
src/backend/probe_rs.rs      ProbeRsBackend（真实 probe-rs 实现）
src/backend/mock.rs          MockBackend（测试/演示用）
src/svd/mod.rs               SvdDatabase、SvdSummary、resolve()
src/svd/parser.rs            SVD 文件解析（svd-parser crate 或最小解析器）
src/mcp/mod.rs               CmsisDapMcp、server instructions、工具路由
src/mcp/tools_probe.rs       探针与会话工具
src/mcp/tools_memory.rs      内存读写工具
src/mcp/tools_core.rs        核心控制工具
src/mcp/tools_dap.rs         DAP 原始访问工具
src/mcp/tools_svd.rs         SVD 命名访问工具
src/mcp/tools_flash.rs       Flash 工具（destructive）
tests/security.rs
tests/session.rs
tests/svd.rs
tests/mcp_handlers.rs
npm/package.json             元包（可选平台二进制）
npm/platforms/cmsis-dap-mcp-win32-x64/package.json
npm/platforms/cmsis-dap-mcp-linux-x64/package.json
npm/platforms/cmsis-dap-mcp-darwin-x64/package.json
docs/book.toml                mdBook 配置
docs/src/*.md                mdBook 页面
.github/workflows/ci.yml
.github/workflows/release.yml
.github/workflows/pages.yml
README.md
LICENSE-APACHE / LICENSE-MIT
```

---

### Task 1: 项目骨架与 MCP hello（CLI + 日志 + stdio）

**Files:**
- Create: `rust-toolchain.toml`, `Cargo.toml`, `src/main.rs`, `src/cli.rs`
- Test: `tests/cli.rs`

**Interfaces:**
- Produces: `cli::AppConfig`（字段 `allow_destructive: bool`、`log_level: String`、`log_file: Option<PathBuf>`、`probe_id: Option<String>`、`protocol: Option<String>`、`speed_khz: Option<u32>`、`target: Option<String>`、`svd: Option<PathBuf>`）；`main()` 启动 rmcp stdio 服务并返回 `Result<(), Box<dyn Error>>`。

- [ ] **Step 1: 写失败测试**

```rust
// tests/cli.rs
use cmsis_dap_mcp::cli::{AppConfig, CliError};

#[test]
fn parses_destructive_flag() {
    let cfg = AppConfig::parse_from(["cmsis-dap-mcp", "--allow-destructive"]).unwrap();
    assert!(cfg.allow_destructive);
}

#[test]
fn rejects_unknown_protocol() {
    let err = AppConfig::parse_from(["cmsis-dap-mcp", "--protocol", "i2c"]).unwrap_err();
    assert!(matches!(err, CliError::InvalidProtocol(_)));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --test cli`
Expected: FAIL，`cmsis_dap_mcp::cli` 不存在。

- [ ] **Step 3: 创建 Cargo 工程与最小实现**

```toml
# Cargo.toml
[package]
name = "cmsis-dap-mcp"
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[dependencies]
rmcp = "0.5"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "0.8"
clap = { version = "4", features = ["derive"] }
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
probe-rs = "0.25"
```

```toml
# rust-toolchain.toml
[toolchain]
channel = "1.97.1"
```

```rust
// src/cli.rs
use clap::Parser;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid protocol: {0}")]
    InvalidProtocol(String),
    #[error(transparent)]
    Clap(#[from] clap::Error),
}

#[derive(Debug, Clone, Parser)]
#[command(name = "cmsis-dap-mcp", about = "CMSIS-DAP MCP server")]
pub struct AppConfig {
    #[arg(long)]
    pub allow_destructive: bool,
    #[arg(long, default_value = "info")]
    pub log_level: String,
    #[arg(long)]
    pub log_file: Option<PathBuf>,
    #[arg(long)]
    pub probe_id: Option<String>,
    #[arg(long)]
    pub protocol: Option<String>,
    #[arg(long)]
    pub speed_khz: Option<u32>,
    #[arg(long)]
    pub target: Option<String>,
    #[arg(long)]
    pub svd: Option<PathBuf>,
}

impl AppConfig {
    pub fn parse_from<I, T>(args: I) -> Result<Self, CliError>
    where I: IntoIterator<Item = T>, T: Into<std::ffi::OsString> + Clone {
        let cfg = AppConfig::parse_from(args);
        if let Some(p) = &cfg.protocol {
            if p != "swd" && p != "jtag" {
                return Err(CliError::InvalidProtocol(p.clone()));
            }
        }
        Ok(cfg)
    }
}
```

```rust
// src/main.rs
use cmsis_dap_mcp::cli::AppConfig;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = AppConfig::parse_from(std::env::args_os())?;
    let filter = EnvFilter::try_new(&cfg.log_level).unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).with_writer(std::io::stderr).init();
    tracing::info!("starting cmsis-dap-mcp (destructive={})", cfg.allow_destructive);
    Ok(())
}
```

- [ ] **Step 4: 运行测试与构建确认通过**

Run: `cargo test --test cli` 与 `cargo build`
Expected: PASS；`target/debug/cmsis-dap-mcp.exe` 生成。

- [ ] **Step 5: 提交**

```bash
git add rust-toolchain.toml Cargo.toml src/main.rs src/cli.rs tests/cli.rs Cargo.lock
git commit -m "feat: scaffold cargo project with cli and logging"
```
---

### Task 2: 错误类型与安全策略

**Files:**
- Create: `src/error.rs`, `src/security.rs`
- Test: `tests/security.rs`

**Interfaces:**
- Consumes: 无。
- Produces:
  - `error::ErrorCode`（`ProbeNotFound`、`ConnectFailed`、`NotConnected`、`ProtocolError`、`Timeout`、`MemoryFault`、`SvdNotLoaded`、`UnsupportedFeature`、`DestructiveDisabled`、`InvalidArgument`、`InternalError`）
  - `error::McpError { code: ErrorCode, message: String }`，实现 `Display`、`Error`。
  - `security::SecurityLevel`（`ReadOnly`/`Write`/`Destructive`）
  - `security::SecurityPolicy { allow_destructive: bool }`；`check(&self, level) -> Result<(), McpError>`。

- [ ] **Step 1: 写失败测试**

```rust
// tests/security.rs
use cmsis_dap_mcp::error::ErrorCode;
use cmsis_dap_mcp::security::{SecurityLevel, SecurityPolicy};

#[test]
fn read_only_always_allowed() {
    let p = SecurityPolicy { allow_destructive: false };
    assert!(p.check(SecurityLevel::ReadOnly).is_ok());
}

#[test]
fn destructive_blocked_by_default() {
    let p = SecurityPolicy { allow_destructive: false };
    let err = p.check(SecurityLevel::Destructive).unwrap_err();
    assert_eq!(err.code, ErrorCode::DestructiveDisabled);
}

#[test]
fn destructive_allowed_when_enabled() {
    let p = SecurityPolicy { allow_destructive: true };
    assert!(p.check(SecurityLevel::Destructive).is_ok());
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --test security`
Expected: FAIL，模块不存在。

- [ ] **Step 3: 实现**

```rust
// src/error.rs
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    ProbeNotFound, ConnectFailed, NotConnected, ProtocolError, Timeout,
    MemoryFault, SvdNotLoaded, UnsupportedFeature, DestructiveDisabled,
    InvalidArgument, InternalError,
}

#[derive(Debug, Clone, Error)]
#[error("{code}: {message}")]
pub struct McpError {
    pub code: ErrorCode,
    pub message: String,
}

impl McpError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}
```

```rust
// src/security.rs
use crate::error::{ErrorCode, McpError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel { ReadOnly, Write, Destructive }

#[derive(Debug, Clone)]
pub struct SecurityPolicy { pub allow_destructive: bool }

impl SecurityPolicy {
    pub fn check(&self, level: SecurityLevel) -> Result<(), McpError> {
        match level {
            SecurityLevel::ReadOnly | SecurityLevel::Write => Ok(()),
            SecurityLevel::Destructive if self.allow_destructive => Ok(()),
            SecurityLevel::Destructive => Err(McpError::new(
                ErrorCode::DestructiveDisabled,
                "destructive tools are disabled; start the server with --allow-destructive to enable flash erase/program",
            )),
        }
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --test security`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add src/error.rs src/security.rs tests/security.rs
git commit -m "feat: add error codes and security policy"
```

---

### Task 3: 后端接口、MockBackend 与 probe-rs 探针枚举

**Files:**
- Create: `src/backend/mod.rs`, `src/backend/mock.rs`, `src/backend/probe_rs.rs`
- Test: `tests/backend_mock.rs`

**Interfaces:**
- Consumes: `error::McpError`。
- Produces:
  - `backend::AccessWidth { U8, U16, U32, U64 }`
  - `backend::ProbeInfo { id: String, vendor: String, product: String, serial: Option<String> }`
  - `backend::Protocol { Swd, Jtag }`
  - `backend::ConnectOptions { probe_id: Option<String>, protocol: Protocol, speed_khz: Option<u32>, target: Option<String> }`
  - `backend::TargetInfo { core_type: String, ap_count: usize }`
  - `backend::CoreRegister { Name(String), Number(u16) }`
  - `trait Backend: Send`：方法签名与设计规格第 6 节一一对应；DAP、断点、寄存器、Flash 方法本任务先用 `UnsupportedFeature` 占位，后续任务补齐。
  - `backend::mock::MockBackend::new()`；`ProbeRsBackend::new()`。

- [ ] **Step 1: 写失败测试**

```rust
// tests/backend_mock.rs
use cmsis_dap_mcp::backend::mock::MockBackend;
use cmsis_dap_mcp::backend::{AccessWidth, Backend, ConnectOptions, Protocol};

#[test]
fn mock_lists_one_probe() {
    let b = MockBackend::new();
    assert_eq!(b.list_probes().unwrap().len(), 1);
}

#[test]
fn mock_memory_roundtrip() {
    let mut b = MockBackend::new();
    b.connect(&ConnectOptions { probe_id: None, protocol: Protocol::Swd, speed_khz: None, target: None }).unwrap();
    b.write_memory(0x2000_0000, AccessWidth::U32, &[0xDEAD_BEEF]).unwrap();
    let v = b.read_memory(0x2000_0000, AccessWidth::U32, 1).unwrap();
    assert_eq!(v, vec![0xDEAD_BEEF]);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --test backend_mock`
Expected: FAIL，模块不存在。

- [ ] **Step 3: 定义 Backend trait 与 MockBackend**

```rust
// src/backend/mod.rs
use crate::error::McpError;
pub mod mock;
pub mod probe_rs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessWidth { U8, U16, U32, U64 }

#[derive(Debug, Clone)]
pub struct ProbeInfo { pub id: String, pub vendor: String, pub product: String, pub serial: Option<String> }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol { Swd, Jtag }

#[derive(Debug, Clone)]
pub struct ConnectOptions { pub probe_id: Option<String>, pub protocol: Protocol, pub speed_khz: Option<u32>, pub target: Option<String> }

#[derive(Debug, Clone)]
pub struct TargetInfo { pub core_type: String, pub ap_count: usize }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreRegister { Name(String), Number(u16) }

pub trait Backend: Send {
    fn list_probes(&self) -> Result<Vec<ProbeInfo>, McpError>;
    fn connect(&mut self, opts: &ConnectOptions) -> Result<TargetInfo, McpError>;
    fn disconnect(&mut self) -> Result<(), McpError>;
    fn read_memory(&mut self, address: u64, width: AccessWidth, count: u32) -> Result<Vec<u64>, McpError>;
    fn write_memory(&mut self, address: u64, width: AccessWidth, data: &[u64]) -> Result<(), McpError>;
    fn read_core_register(&mut self, reg: &CoreRegister) -> Result<u64, McpError>;
    fn write_core_register(&mut self, reg: &CoreRegister, value: u64) -> Result<(), McpError>;
    fn halt(&mut self) -> Result<(), McpError>;
    fn resume(&mut self) -> Result<(), McpError>;
    fn step(&mut self) -> Result<(), McpError>;
    fn set_breakpoint(&mut self, address: u64) -> Result<(), McpError>;
    fn clear_breakpoints(&mut self) -> Result<(), McpError>;
    fn list_breakpoints(&mut self) -> Result<Vec<u64>, McpError>;
    fn reset(&mut self) -> Result<(), McpError>;
    fn read_dap(&mut self, address: u32) -> Result<u32, McpError>;
    fn write_dap(&mut self, address: u32, value: u32) -> Result<(), McpError>;
    fn erase_flash(&mut self, address: u64, size: u64) -> Result<(), McpError>;
    fn program_flash(&mut self, address: u64, data: &[u8]) -> Result<(), McpError>;
}
```

```rust
// src/backend/mock.rs
use crate::backend::{AccessWidth, Backend, ConnectOptions, CoreRegister, ProbeInfo, Protocol, TargetInfo};
use crate::error::{ErrorCode, McpError};
use std::collections::HashMap;

pub struct MockBackend {
    memory: HashMap<u64, u64>,
    connected: bool,
}

impl MockBackend {
    pub fn new() -> Self {
        Self { memory: HashMap::new(), connected: false }
    }
}

fn width_bytes(width: AccessWidth) -> u64 {
    match width { AccessWidth::U8 => 1, AccessWidth::U16 => 2, AccessWidth::U32 => 4, AccessWidth::U64 => 8 }
}

impl Backend for MockBackend {
    fn list_probes(&self) -> Result<Vec<ProbeInfo>, McpError> {
        Ok(vec![ProbeInfo { id: "mock".into(), vendor: "mock".into(), product: "mock".into(), serial: None }])
    }
    fn connect(&mut self, _opts: &ConnectOptions) -> Result<TargetInfo, McpError> {
        self.connected = true;
        Ok(TargetInfo { core_type: "Cortex-M0".into(), ap_count: 1 })
    }
    fn disconnect(&mut self) -> Result<(), McpError> { self.connected = false; Ok(()) }
    fn read_memory(&mut self, address: u64, width: AccessWidth, count: u32) -> Result<Vec<u64>, McpError> {
        if !self.connected { return Err(McpError::new(ErrorCode::NotConnected, "no active session")); }
        let step = width_bytes(width);
        Ok((0..count).map(|i| *self.memory.get(&(address + i as u64 * step)).unwrap_or(&0)).collect())
    }
    fn write_memory(&mut self, address: u64, width: AccessWidth, data: &[u64]) -> Result<(), McpError> {
        if !self.connected { return Err(McpError::new(ErrorCode::NotConnected, "no active session")); }
        for (i, v) in data.iter().enumerate() {
            self.memory.insert(address + i as u64 * width_bytes(width), *v);
        }
        Ok(())
    }
    // 其余方法：未连接返回 NotConnected；已连接返回 UnsupportedFeature（Task 7/8/10 补齐）
}
```

- [ ] **Step 4: probe-rs 枚举实现（仅 list_probes + 占位）**

```rust
// src/backend/probe_rs.rs
use crate::backend::{Backend, ProbeInfo};
use crate::error::{ErrorCode, McpError};
use probe_rs::Probe;

pub struct ProbeRsBackend;

impl ProbeRsBackend {
    pub fn new() -> Self { Self }
}

impl Backend for ProbeRsBackend {
    fn list_probes(&self) -> Result<Vec<ProbeInfo>, McpError> {
        let probes = Probe::open_all().map_err(|e| McpError::new(ErrorCode::ProbeNotFound, e.to_string()))?;
        Ok(probes.into_iter().map(|p| ProbeInfo {
            id: p.info.product_name.clone(),
            vendor: p.info.vendor_name.clone(),
            product: p.info.product_name.clone(),
            serial: p.info.unique_id.clone(),
        }).collect())
    }
    // 其余方法 Task 4/7/8/10 补齐；当前返回 UnsupportedFeature
}
```

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test --test backend_mock`
Expected: PASS。

- [ ] **Step 6: 提交**

```bash
git add src/backend/mod.rs src/backend/mock.rs src/backend/probe_rs.rs tests/backend_mock.rs
git commit -m "feat: define backend trait with mock and probe-rs probe enumeration"
```

---

### Task 4: probe-rs 连接与内存读写（真实后端）

**Files:**
- Modify: `src/backend/probe_rs.rs`
- Test: `tests/backend_probe_rs.rs`（无硬件：仅验证错误路径；实机验证在 Task 16）

**Interfaces:**
- Consumes: Task 3 的 `Backend` trait。
- Produces: `ProbeRsBackend` 完整实现 `connect`、`disconnect`、`read_memory`、`write_memory`。

- [ ] **Step 1: 写失败测试（错误路径）**

```rust
// tests/backend_probe_rs.rs
use cmsis_dap_mcp::backend::probe_rs::ProbeRsBackend;
use cmsis_dap_mcp::backend::{AccessWidth, Backend};
use cmsis_dap_mcp::error::ErrorCode;

#[test]
fn memory_read_without_connect_fails() {
    let mut b = ProbeRsBackend::new();
    let err = b.read_memory(0x2000_0000, AccessWidth::U32, 1).unwrap_err();
    assert_eq!(err.code, ErrorCode::NotConnected);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --test backend_probe_rs`
Expected: FAIL，未实现 `read_memory`。

- [ ] **Step 3: 实现 probe-rs 连接与内存访问**

```rust
// src/backend/probe_rs.rs（追加）
use crate::backend::{AccessWidth, Backend, ConnectOptions, CoreRegister, Protocol, TargetInfo};
use crate::error::{ErrorCode, McpError};
use probe_rs::{MemoryInterface, Permissions, Probe, Session};

pub struct ProbeRsBackend {
    session: Option<Session>,
    core_index: usize,
}

impl ProbeRsBackend {
    pub fn new() -> Self { Self { session: None, core_index: 0 } }

    fn core(&mut self) -> Result<probe_rs::Core<'_>, McpError> {
        self.session.as_mut()
            .ok_or_else(|| McpError::new(ErrorCode::NotConnected, "no active session"))?
            .core(self.core_index)
            .map_err(|e| McpError::new(ErrorCode::ConnectFailed, e.to_string()))
    }
}

impl Backend for ProbeRsBackend {
    // list_probes 沿用 Task 3

    fn connect(&mut self, opts: &ConnectOptions) -> Result<TargetInfo, McpError> {
        if self.session.is_some() { self.disconnect()?; }
        let mut probe = match &opts.probe_id {
            Some(id) => Probe::open(id).map_err(|e| McpError::new(ErrorCode::ProbeNotFound, e.to_string()))?,
            None => {
                let mut probes = Probe::open_all().map_err(|e| McpError::new(ErrorCode::ProbeNotFound, e.to_string()))?;
                probes.pop().ok_or_else(|| McpError::new(ErrorCode::ProbeNotFound, "no probe found"))?
            }
        };
        probe.set_speed_khz(opts.speed_khz.unwrap_or(1000)).ok();
        match opts.protocol {
            Protocol::Swd => probe.select_protocol(probe_rs::WireProtocol::Swd).map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?,
            Protocol::Jtag => probe.select_protocol(probe_rs::WireProtocol::Jtag).map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?,
        }
        let session = match &opts.target {
            Some(name) => probe.attach(name, Permissions::default()).map_err(|e| McpError::new(ErrorCode::ConnectFailed, e.to_string()))?,
            None => probe.attach_to_unspecified().and_then(|_| probe.attach(probe_rs::config::TargetSelector::Auto, Permissions::default()))
                .map_err(|e| McpError::new(ErrorCode::ConnectFailed, e.to_string()))?,
        };
        let core_type = format!("{:?}", session.target().cores()[self.core_index].core_type());
        let ap_count = session.target().memory_map().iter().count();
        self.session = Some(session);
        Ok(TargetInfo { core_type, ap_count })
    }

    fn disconnect(&mut self) -> Result<(), McpError> {
        self.session.take();
        Ok(())
    }

    fn read_memory(&mut self, address: u64, width: AccessWidth, count: u32) -> Result<Vec<u64>, McpError> {
        let mut core = self.core()?;
        match width {
            AccessWidth::U32 => {
                let mut buf = vec![0u32; count as usize];
                core.read_32(address as u32, &mut buf).map_err(|e| McpError::new(ErrorCode::MemoryFault, e.to_string()))?;
                Ok(buf.iter().map(|v| *v as u64).collect())
            }
            _ => Err(McpError::new(ErrorCode::UnsupportedFeature, format!("width {width:?} not supported yet"))),
        }
    }

    fn write_memory(&mut self, address: u64, width: AccessWidth, data: &[u64]) -> Result<(), McpError> {
        let mut core = self.core()?;
        match width {
            AccessWidth::U32 => {
                let buf: Vec<u32> = data.iter().map(|v| *v as u32).collect();
                core.write_32(address as u32, &buf).map_err(|e| McpError::new(ErrorCode::MemoryFault, e.to_string()))?;
                Ok(())
            }
            _ => Err(McpError::new(ErrorCode::UnsupportedFeature, format!("width {width:?} not supported yet"))),
        }
    }
    // 其余方法 Task 7/8/10 补齐
}
```

- [ ] **Step 4: 核对 probe-rs API 并运行测试**

Run: `cargo test --test backend_probe_rs`；若 API 名与当前 probe-rs 版本不一致，以 docs.rs 当前版本为准修正后重跑。
Expected: PASS；`cargo build` 通过。

- [ ] **Step 5: 提交**

```bash
git add src/backend/probe_rs.rs tests/backend_probe_rs.rs
git commit -m "feat: probe-rs connect and memory read/write"
```

---

### Task 5: 会话管理（SessionManager）

**Files:**
- Create: `src/session.rs`
- Test: `tests/session.rs`

**Interfaces:**
- Consumes: `Backend` trait、`McpError`。
- Produces:
  - `session::SessionManager { backend: Box<dyn Backend>, connected: Option<TargetInfo>, svd: Option<SvdDatabase> }`
  - 方法：`new(Box<dyn Backend>)`、`connect(&mut self, &ConnectOptions) -> Result<TargetInfo, McpError>`（自动断开旧会话）、`disconnect(&mut self) -> Result<(), McpError>`、`ensure_connected(&self) -> Result<(), McpError>`、`backend(&mut self) -> &mut dyn Backend`、`load_svd(&mut self, &Path) -> Result<SvdSummary, McpError>`（Task 9 前返回 `UnsupportedFeature`）、`svd(&self) -> Result<&SvdDatabase, McpError>`、`target_info(&self) -> Option<&TargetInfo>`。

- [ ] **Step 1: 写失败测试**

```rust
// tests/session.rs
use cmsis_dap_mcp::backend::mock::MockBackend;
use cmsis_dap_mcp::backend::{AccessWidth, Backend, ConnectOptions, Protocol};
use cmsis_dap_mcp::error::ErrorCode;
use cmsis_dap_mcp::session::SessionManager;

#[test]
fn connect_sets_state_and_auto_disconnects() {
    let mut sm = SessionManager::new(Box::new(MockBackend::new()));
    let opts = ConnectOptions { probe_id: None, protocol: Protocol::Swd, speed_khz: None, target: None };
    sm.connect(&opts).unwrap();
    sm.connect(&opts).unwrap();
    sm.ensure_connected().unwrap();
}

#[test]
fn memory_before_connect_fails() {
    let mut sm = SessionManager::new(Box::new(MockBackend::new()));
    let err = sm.backend().read_memory(0, AccessWidth::U32, 1).unwrap_err();
    assert_eq!(err.code, ErrorCode::NotConnected);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --test session`
Expected: FAIL，`session` 模块不存在。

- [ ] **Step 3: 实现 SessionManager**

```rust
// src/session.rs
use crate::backend::{Backend, ConnectOptions, TargetInfo};
use crate::error::{ErrorCode, McpError};
use crate::svd::{SvdDatabase, SvdSummary};
use std::path::Path;

pub struct SessionManager {
    backend: Box<dyn Backend>,
    connected: Option<TargetInfo>,
    svd: Option<SvdDatabase>,
}

impl SessionManager {
    pub fn new(backend: Box<dyn Backend>) -> Self {
        Self { backend, connected: None, svd: None }
    }
    pub fn connect(&mut self, opts: &ConnectOptions) -> Result<TargetInfo, McpError> {
        if self.connected.is_some() {
            tracing::info!("auto-disconnecting previous session before connect");
            self.disconnect()?;
        }
        let info = self.backend.connect(opts)?;
        self.connected = Some(info.clone());
        Ok(info)
    }
    pub fn disconnect(&mut self) -> Result<(), McpError> {
        if self.connected.is_some() {
            self.backend.disconnect()?;
            self.connected = None;
        }
        Ok(())
    }
    pub fn ensure_connected(&self) -> Result<(), McpError> {
        if self.connected.is_none() {
            return Err(McpError::new(ErrorCode::NotConnected, "call connect first"));
        }
        Ok(())
    }
    pub fn backend(&mut self) -> &mut dyn Backend { self.backend.as_mut() }
    pub fn load_svd(&mut self, path: &Path) -> Result<SvdSummary, McpError> {
        let db = SvdDatabase::load(path)?;
        let summary = db.summary();
        self.svd = Some(db);
        Ok(summary)
    }
    pub fn svd(&self) -> Result<&SvdDatabase, McpError> {
        self.svd.as_ref().ok_or_else(|| McpError::new(ErrorCode::SvdNotLoaded, "load an SVD file with load_svd first"))
    }
    pub fn target_info(&self) -> Option<&TargetInfo> { self.connected.as_ref() }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --test session`
Expected: PASS（`SvdDatabase` 在 Task 9 前先用最小桩编译）。

- [ ] **Step 5: 提交**

```bash
git add src/session.rs tests/session.rs
git commit -m "feat: session manager with auto-disconnect"
```
---

### Task 6: 内存读写 MCP 工具

**Files:**
- Create: `src/mcp/mod.rs`, `src/mcp/tools_memory.rs`
- Test: `tests/mcp_handlers.rs`

**Interfaces:**
- Consumes: `SessionManager`、`SecurityPolicy`。
- Produces:
  - `mcp::CmsisDapMcp { session: SessionManager, policy: SecurityPolicy }`，`new(session, policy)`。
  - 工具：`read_memory(address: u64, width: "u8"|"u16"|"u32"|"u64", count: u32) -> structured {address, width, count, values}`，等级 ReadOnly；`write_memory(address, width, values: Vec<u64>) -> structured {address, width, written}`，等级 Write。
  - rmcp 宏语法（annotations）先按示例实现，编译不通过时以 rmcp `examples/` 与 docs.rs 当前语法修正。

- [ ] **Step 1: 写失败测试（直接调用 handler 方法）**

```rust
// tests/mcp_handlers.rs
use cmsis_dap_mcp::backend::mock::MockBackend;
use cmsis_dap_mcp::mcp::{CmsisDapMcp, ReadMemoryParams};
use cmsis_dap_mcp::security::SecurityPolicy;
use cmsis_dap_mcp::session::SessionManager;
use rmcp::handler::server::wrapper::Parameters;

#[tokio::test]
async fn read_memory_returns_mock_values() {
    let mut mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), SecurityPolicy { allow_destructive: false });
    let params = ReadMemoryParams { address: 0x2000_0000, width: "u32".into(), count: 1 };
    let res = mcp.read_memory(Parameters(params)).await;
    assert!(!res.is_error.unwrap_or(true));
    let structured = res.structured_content.unwrap();
    assert_eq!(structured["values"][0], 0);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --test mcp_handlers`
Expected: FAIL，模块不存在。

- [ ] **Step 3: 实现 MCP 结构体与内存工具**

```rust
// src/mcp/mod.rs
pub mod tools_memory;

use crate::security::SecurityPolicy;
use crate::session::SessionManager;
use rmcp::tool_router;

#[derive(Clone)]
pub struct CmsisDapMcp {
    pub session: SessionManager,
    pub policy: SecurityPolicy,
}

impl CmsisDapMcp {
    pub fn new(session: SessionManager, policy: SecurityPolicy) -> Self {
        Self { session, policy }
    }
}

#[tool_router(server_handler)]
impl CmsisDapMcp {
    // 各模块工具通过 impl 块合并注册；模块间共享此结构体
}
```

```rust
// src/mcp/tools_memory.rs
use crate::backend::AccessWidth;
use crate::error::ErrorCode;
use crate::mcp::CmsisDapMcp;
use crate::security::SecurityLevel;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::ContentBlock;
use rmcp::tool;
use rmcp::CallToolResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ReadMemoryParams {
    pub address: u64,
    pub width: String,
    #[schemars(default = "default_count")]
    pub count: u32,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WriteMemoryParams {
    pub address: u64,
    pub width: String,
    pub values: Vec<u64>,
}

fn default_count() -> u32 { 1 }

fn parse_width(s: &str) -> Option<AccessWidth> {
    match s {
        "u8" => Some(AccessWidth::U8),
        "u16" => Some(AccessWidth::U16),
        "u32" => Some(AccessWidth::U32),
        "u64" => Some(AccessWidth::U64),
        _ => None,
    }
}

pub fn error_result(code: ErrorCode, message: String) -> CallToolResult {
    CallToolResult::structured_error(serde_json::json!({
        "code": format!("{code:?}"),
        "message": message,
    }))
}

impl CmsisDapMcp {
    #[tool(description = "Read memory from the connected target. width is one of u8/u16/u32/u64; count is the number of elements.", annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
    pub async fn read_memory(&mut self, Parameters(params): Parameters<ReadMemoryParams>) -> CallToolResult {
        let _ = self.policy.check(SecurityLevel::ReadOnly);
        let width = match parse_width(&params.width) {
            Some(w) => w,
            None => return error_result(ErrorCode::InvalidArgument, "width must be u8/u16/u32/u64".into()),
        };
        match self.session.backend().read_memory(params.address, width, params.count) {
            Ok(values) => CallToolResult::structured(serde_json::json!({
                "address": params.address,
                "width": params.width,
                "count": params.count,
                "values": values,
            })),
            Err(e) => error_result(e.code, e.message),
        }
    }

    #[tool(description = "Write memory on the connected target. width is one of u8/u16/u32/u64; values are the elements to write.", annotations(read_only_hint = false, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
    pub async fn write_memory(&mut self, Parameters(params): Parameters<WriteMemoryParams>) -> CallToolResult {
        let _ = self.policy.check(SecurityLevel::Write);
        let width = match parse_width(&params.width) {
            Some(w) => w,
            None => return error_result(ErrorCode::InvalidArgument, "width must be u8/u16/u32/u64".into()),
        };
        match self.session.backend().write_memory(params.address, width, &params.values) {
            Ok(()) => CallToolResult::structured(serde_json::json!({
                "address": params.address,
                "width": params.width,
                "written": params.values.len(),
            })),
            Err(e) => error_result(e.code, e.message),
        }
    }
}
```

- [ ] **Step 4: 修正 rmcp 宏/错误映射后运行测试**

Run: `cargo test --test mcp_handlers`
说明：若 rmcp 0.5 的 annotation 宏或 `CallToolResult` API 与示例不同，先查看本地 cargo registry 中 rmcp `examples/` 或 docs.rs 当前文档修正语法，再重跑。
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add src/mcp/mod.rs src/mcp/tools_memory.rs tests/mcp_handlers.rs
git commit -m "feat: memory read/write MCP tools"
```

---

### Task 7: 核心控制工具（寄存器、halt/resume/step、断点、复位）

**Files:**
- Modify: `src/backend/mock.rs`, `src/backend/probe_rs.rs`
- Create: `src/mcp/tools_core.rs`
- Test: `tests/mcp_handlers.rs`（追加）

**Interfaces:**
- Consumes: Task 3 的 `Backend` 方法签名。
- Produces: 工具 `read_core_register(register: String|u16)`、`write_core_register`、`halt`、`resume`、`step`、`set_breakpoint(address)`、`clear_breakpoints`、`list_breakpoints`、`reset`；等级按设计规格第 6 节；`reset` 标注 destructiveHint。

- [ ] **Step 1: 为 MockBackend 补齐核心控制并写测试**

```rust
// tests/mcp_handlers.rs 追加
use cmsis_dap_mcp::backend::{ConnectOptions, Protocol};
use cmsis_dap_mcp::mcp::{HaltParams, ResumeParams, SetBreakpointParams};

#[tokio::test]
async fn core_control_flow_with_mock() {
    let mut mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), SecurityPolicy { allow_destructive: false });
    mcp.session.connect(&ConnectOptions { probe_id: None, protocol: Protocol::Swd, speed_khz: None, target: None }).unwrap();
    assert!(!mcp.halt(Parameters(HaltParams {})).await.is_error.unwrap_or(true));
    assert!(!mcp.resume(Parameters(ResumeParams {})).await.is_error.unwrap_or(true));
    assert!(!mcp.set_breakpoint(Parameters(SetBreakpointParams { address: 0x0800_0000 })).await.is_error.unwrap_or(true));
    assert_eq!(mcp.session.backend().list_breakpoints().unwrap(), vec![0x0800_0000]);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --test mcp_handlers`
Expected: FAIL，工具方法未定义。

- [ ] **Step 3: 补齐 MockBackend 并实现工具**

MockBackend 新增字段 `halted: bool`、`breakpoints: Vec<u64>`；`halt/resume/step` 更新状态，`set_breakpoint` 排序去重，`list_breakpoints` 返回副本，`reset` 清断点并置 `halted=false`。

```rust
// src/mcp/tools_core.rs（节选，完整实现覆盖 9 个工具）
use crate::mcp::CmsisDapMcp;
use crate::security::SecurityLevel;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::ContentBlock;
use rmcp::tool;
use rmcp::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct HaltParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ResumeParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SetBreakpointParams { pub address: u64 }

impl CmsisDapMcp {
    #[tool(description = "Halt the connected core.", annotations(read_only_hint = false, destructive_hint = false, idempotent_hint = false, open_world_hint = false))]
    pub async fn halt(&mut self, Parameters(_): Parameters<HaltParams>) -> CallToolResult {
        let _ = self.policy.check(SecurityLevel::Write);
        match self.session.backend().halt() {
            Ok(()) => CallToolResult::success(vec![ContentBlock::text("core halted")]),
            Err(e) => super::tools_memory::error_result(e.code, e.message),
        }
    }

    #[tool(description = "Set a hardware breakpoint at the given address.", annotations(read_only_hint = false, destructive_hint = false, idempotent_hint = true, open_world_hint = false))]
    pub async fn set_breakpoint(&mut self, Parameters(p): Parameters<SetBreakpointParams>) -> CallToolResult {
        let _ = self.policy.check(SecurityLevel::Write);
        match self.session.backend().set_breakpoint(p.address) {
            Ok(()) => CallToolResult::structured(serde_json::json!({ "address": p.address, "set": true })),
            Err(e) => super::tools_memory::error_result(e.code, e.message),
        }
    }
}
```

- [ ] **Step 4: probe-rs 核心控制实现**

`ProbeRsBackend`：`halt()` -> `core.halt()`；`resume()` -> `core.run()`；`step()` -> `core.step()`；`read_core_register(Name(n))` -> 按名字查询核心寄存器组（API 以当前 probe-rs 为准）；`set_breakpoint` -> `core.set_hw_breakpoint(address)`；`list_breakpoints` -> 若当前版本无查询 API，维护本地 `Vec<u64>` 影子列表；`reset` -> `core.reset()`。

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test --test mcp_handlers` 与 `cargo build`
Expected: PASS；构建通过。

- [ ] **Step 6: 提交**

```bash
git add src/backend/mock.rs src/backend/probe_rs.rs src/mcp/tools_core.rs tests/mcp_handlers.rs
git commit -m "feat: core control tools halt/resume/step/registers/breakpoints/reset"
```

---

### Task 8: DAP 原始访问工具

**Files:**
- Modify: `src/backend/mock.rs`, `src/backend/probe_rs.rs`
- Create: `src/mcp/tools_dap.rs`
- Test: `tests/mcp_handlers.rs`（追加）

**Interfaces:**
- Produces: `read_dap(address: u32) -> {address, value}`（ReadOnly）；`write_dap(address: u32, value: u32)`（Write）。地址为 DP/AP 寄存器地址（含 APSEL，如 `0x010000FC`）。

- [ ] **Step 1: 写测试**

```rust
// tests/mcp_handlers.rs 追加
use cmsis_dap_mcp::mcp::{ReadDapParams, WriteDapParams};

#[tokio::test]
async fn dap_read_write_with_mock() {
    let mut mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), SecurityPolicy { allow_destructive: false });
    mcp.session.connect(&ConnectOptions { probe_id: None, protocol: Protocol::Swd, speed_khz: None, target: None }).unwrap();
    let res = mcp.write_dap(Parameters(WriteDapParams { address: 0x4, value: 0x1 })).await;
    assert!(!res.is_error.unwrap_or(true));
    let res = mcp.read_dap(Parameters(ReadDapParams { address: 0x4 })).await;
    assert_eq!(res.structured_content.unwrap()["value"], 0x1);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --test mcp_handlers`
Expected: FAIL，工具未定义。

- [ ] **Step 3: 实现**

MockBackend 新增 `dap: HashMap<u32, u32>`；`read_dap/write_dap` 读写该表。
ProbeRsBackend：`read_dap` 使用 DAP 原始访问 API（`RawDapAccess::raw_read_register` 或 DP/AP 寄存器接口，以当前 probe-rs 版本为准），错误映射到 `ProtocolError`。
MCP 工具实现与 Task 6 同构（ReadOnly/Write 等级、结构化输出）。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --test mcp_handlers` 与 `cargo build`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add src/backend/mock.rs src/backend/probe_rs.rs src/mcp/tools_dap.rs tests/mcp_handlers.rs
git commit -m "feat: raw DP/AP read-write tools"
```

---

### Task 9: SVD 加载与命名外设访问

**Files:**
- Create: `src/svd/mod.rs`, `src/svd/parser.rs`, `src/mcp/tools_svd.rs`
- Test: `tests/svd.rs`, `tests/mcp_handlers.rs`（追加）

**Interfaces:**
- Consumes: Task 2 的 `McpError`、Task 5 的 `SessionManager::load_svd`。
- Produces:
  - `svd::SvdSummary { name: String, peripherals: usize }`
  - `svd::SvdDatabase::load(&Path) -> Result<Self, McpError>`、`summary()`、`list_peripherals() -> Vec<String>`、`resolve(peripheral: &str, register: &str, field: Option<&str>) -> Result<(u64, Option<(u32, u32)>), McpError>`（返回值 = (绝对地址, (mask, shift))）。
  - 工具：`load_svd(path: String)`（Write）、`list_peripherals`（ReadOnly）、`read_peripheral(peripheral, register, field?)`（ReadOnly）、`write_peripheral(peripheral, register, value, field?)`（Write）。

- [ ] **Step 1: 写失败测试（用小 SVD 字符串）**

```rust
// tests/svd.rs
use cmsis_dap_mcp::svd::SvdDatabase;
use std::io::Write;

const MINI_SVD: &str = r#"<?xml version="1.0"?>
<device schemaVersion="1.1"><peripherals>
<peripheral><name>GPIOA</name><baseAddress>0x48000000</baseAddress>
<registers><register><name>ODR</name><addressOffset>0x14</addressOffset><size>32</size><access>read-write</access>
<fields><field><name>ODR0</name><bitOffset>0</bitOffset><bitWidth>1</bitWidth></field></fields>
</register></registers></peripheral>
</peripherals></device>"#;

#[test]
fn parses_mini_svd_and_resolves() {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(MINI_SVD.as_bytes()).unwrap();
    let db = SvdDatabase::load(f.path()).unwrap();
    assert_eq!(db.list_peripherals(), vec!["GPIOA"]);
    let (addr, field) = db.resolve("GPIOA", "ODR", Some("ODR0")).unwrap();
    assert_eq!(addr, 0x4800_0014);
    assert_eq!(field, Some((0x1, 0)));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --test svd`
Expected: FAIL，模块不存在。

- [ ] **Step 3: 选择解析器并实现**

优先 `cargo add svd-parser`；若该 crate 的当前版本无法解析上述最小 SVD，则用 `quick-xml` 实现最小解析器（仅外设/寄存器/字段三级）。

```rust
// src/svd/mod.rs（svd-parser 路线）
use crate::error::{ErrorCode, McpError};
use std::path::Path;

pub struct SvdSummary { pub name: String, pub peripherals: usize }

pub struct SvdDatabase {
    name: String,
    peripherals: Vec<SvdPeripheral>,
}

struct SvdPeripheral { name: String, base: u64, registers: Vec<SvdRegister> }
struct SvdRegister { name: String, offset: u64, fields: Vec<SvdField> }
struct SvdField { name: String, offset: u32, width: u32 }

impl SvdDatabase {
    pub fn load(path: &Path) -> Result<Self, McpError> {
        let text = std::fs::read_to_string(path).map_err(|e| McpError::new(ErrorCode::SvdNotLoaded, e.to_string()))?;
        let parsed = svd_parser::parse(&text).map_err(|e| McpError::new(ErrorCode::SvdNotLoaded, e.to_string()))?;
        let peripherals = parsed.peripherals.iter().map(|p| SvdPeripheral {
            name: p.name.clone(),
            base: p.base_address,
            registers: p.registers.iter().flat_map(|r| r.clone().into_iter()).map(|r| SvdRegister {
                name: r.name.clone(),
                offset: r.address_offset,
                fields: r.fields.unwrap_or_default().iter().map(|f| SvdField {
                    name: f.name.clone(), offset: f.bit_offset, width: f.bit_width,
                }).collect(),
            }).collect(),
        }).collect();
        Ok(Self { name: parsed.name.clone(), peripherals })
    }
    pub fn summary(&self) -> SvdSummary { SvdSummary { name: self.name.clone(), peripherals: self.peripherals.len() } }
    pub fn list_peripherals(&self) -> Vec<String> { self.peripherals.iter().map(|p| p.name.clone()).collect() }
    pub fn resolve(&self, peripheral: &str, register: &str, field: Option<&str>) -> Result<(u64, Option<(u32, u32)>), McpError> {
        let p = self.peripherals.iter().find(|p| p.name.eq_ignore_ascii_case(peripheral))
            .ok_or_else(|| McpError::new(ErrorCode::InvalidArgument, format!("peripheral {peripheral} not found")))?;
        let r = p.registers.iter().find(|r| r.name.eq_ignore_ascii_case(register))
            .ok_or_else(|| McpError::new(ErrorCode::InvalidArgument, format!("register {register} not found")))?;
        let addr = p.base + r.offset;
        match field {
            None => Ok((addr, None)),
            Some(name) => {
                let f = r.fields.iter().find(|f| f.name.eq_ignore_ascii_case(name))
                    .ok_or_else(|| McpError::new(ErrorCode::InvalidArgument, format!("field {name} not found")))?;
                let mask = ((1u64 << f.width) - 1) as u32;
                Ok((addr, Some((mask, f.offset))))
            }
        }
    }
}
```

- [ ] **Step 4: 实现命名访问工具**

`read_peripheral`：`resolve` 后调用 `read_memory`（ReadOnly）；`write_peripheral`：字段写入走读取-修改-写，寄存器写入直接写（Write）；`load_svd`：调用 `session.load_svd`。

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test --test svd`、`cargo test --test mcp_handlers`、`cargo build`
Expected: PASS。

- [ ] **Step 6: 提交**

```bash
git add src/svd src/mcp/tools_svd.rs tests/svd.rs tests/mcp_handlers.rs Cargo.toml Cargo.lock
git commit -m "feat: SVD loading and named peripheral access"
```
---

### Task 10: Flash 工具（destructive）

**Files:**
- Modify: `src/backend/mock.rs`, `src/backend/probe_rs.rs`
- Create: `src/mcp/tools_flash.rs`
- Test: `tests/mcp_handlers.rs`（追加）

**Interfaces:**
- Consumes: Task 2 的 `SecurityPolicy`。
- Produces: `erase_flash(address: u64, size: u64)`、`program_flash(address: u64, data: Vec<u8>)`，等级 `Destructive`；未开 `--allow-destructive` 时返回 `DestructiveDisabled`。

- [ ] **Step 1: 写失败测试（含安全门）**

```rust
// tests/mcp_handlers.rs 追加
use cmsis_dap_mcp::mcp::{EraseFlashParams, ProgramFlashParams};

#[tokio::test]
async fn flash_blocked_without_flag() {
    let mut mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), SecurityPolicy { allow_destructive: false });
    let res = mcp.erase_flash(Parameters(EraseFlashParams { address: 0x0800_0000, size: 0x1000 })).await;
    let structured = res.structured_content.unwrap_or_default();
    assert_eq!(structured["code"], "DestructiveDisabled");
}

#[tokio::test]
async fn flash_works_with_flag() {
    let mut mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), SecurityPolicy { allow_destructive: true });
    mcp.session.connect(&ConnectOptions { probe_id: None, protocol: Protocol::Swd, speed_khz: None, target: None }).unwrap();
    let res = mcp.program_flash(Parameters(ProgramFlashParams { address: 0x0800_0000, data: vec![0xAA, 0xBB] })).await;
    assert!(!res.is_error.unwrap_or(true));
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --test mcp_handlers`
Expected: FAIL，工具未定义。

- [ ] **Step 3: 实现**

工具先调用 `policy.check(SecurityLevel::Destructive)?`，再调 `session.backend()`。MockBackend：`program_flash` 将字节写入内存，`erase_flash` 置 0xFF。
ProbeRsBackend：要求连接时指定含 Flash 算法的 target 名；`program_flash` 使用 `probe_rs::flashing` 的 FlashLoader/下载接口（以当前版本 API 为准），`erase_flash` 使用对应 Flash 实例擦除。

- [ ] **Step 4: 运行测试确认通过**

Run: `cargo test --test mcp_handlers` 与 `cargo build`
Expected: PASS。

- [ ] **Step 5: 提交**

```bash
git add src/backend/mock.rs src/backend/probe_rs.rs src/mcp/tools_flash.rs tests/mcp_handlers.rs
git commit -m "feat: destructive flash tools with security gate"
```

---

### Task 11: 探针与会话 MCP 工具 + server instructions

**Files:**
- Create: `src/mcp/tools_probe.rs`
- Modify: `src/mcp/mod.rs`, `src/main.rs`
- Test: `tests/mcp_handlers.rs`（追加）

**Interfaces:**
- Consumes: Task 5 的 `SessionManager`。
- Produces: `list_probes`、`get_probe_info(probe_id?)`、`connect(probe_id?, protocol?, speed_khz?, target?)`、`disconnect`、`get_target_info`；server instructions 常量 `SERVER_INSTRUCTIONS: &str`（首 512 字符自包含）。

- [ ] **Step 1: 写测试**

```rust
// tests/mcp_handlers.rs 追加
use cmsis_dap_mcp::mcp::{ConnectParams, DisconnectParams, ListProbesParams};

#[tokio::test]
async fn connect_disconnect_flow() {
    let mut mcp = CmsisDapMcp::new(SessionManager::new(Box::new(MockBackend::new())), SecurityPolicy { allow_destructive: false });
    let res = mcp.list_probes(Parameters(ListProbesParams {})).await;
    assert!(!res.is_error.unwrap_or(true));
    assert_eq!(res.structured_content.unwrap()["probes"].as_array().unwrap().len(), 1);
    let res = mcp.connect(Parameters(ConnectParams { probe_id: None, protocol: Some("swd".into()), speed_khz: None, target: None })).await;
    assert!(!res.is_error.unwrap_or(true));
    let res = mcp.disconnect(Parameters(DisconnectParams {})).await;
    assert!(!res.is_error.unwrap_or(true));
}

#[test]
fn instructions_are_self_contained() {
    assert!(cmsis_dap_mcp::mcp::SERVER_INSTRUCTIONS.len() >= 512);
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test --test mcp_handlers`
Expected: FAIL，工具与常量未定义。

- [ ] **Step 3: 实现**

工具实现与 Task 6 同构；`connect` 先解析协议字符串为 `Protocol`（非法值返回 `InvalidArgument`），再调用 `session.connect`。
`SERVER_INSTRUCTIONS` 内容（需 >=512 字符）：说明三级安全；连接前先 `list_probes`；`connect` 后可 `read_memory`；命名外设需先 `load_svd`；Flash 需 `--allow-destructive`；所有工具返回结构化 JSON。

- [ ] **Step 4: main 组装并启动服务**

```rust
// src/main.rs（替换占位）
use cmsis_dap_mcp::backend::probe_rs::ProbeRsBackend;
use cmsis_dap_mcp::mcp::CmsisDapMcp;
use cmsis_dap_mcp::security::SecurityPolicy;
use cmsis_dap_mcp::session::SessionManager;
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = AppConfig::parse_from(std::env::args_os())?;
    // 日志初始化同 Task 1
    let session = SessionManager::new(Box::new(ProbeRsBackend::new()));
    let policy = SecurityPolicy { allow_destructive: cfg.allow_destructive };
    let mcp = CmsisDapMcp::new(session, policy);
    mcp.serve((tokio::io::stdin(), tokio::io::stdout())).await?;
    Ok(())
}
```

- [ ] **Step 5: 运行测试与构建确认通过**

Run: `cargo test`、`cargo build --release`
Expected: 全部 PASS；`target/release/cmsis-dap-mcp.exe` 生成。

- [ ] **Step 6: 提交**

```bash
git add src/mcp src/main.rs tests/mcp_handlers.rs
git commit -m "feat: probe/session tools and server instructions"
```

---

### Task 12: README 与 npm 双入口包装

**Files:**
- Create: `README.md`, `npm/package.json`, `npm/bin/cmsis-dap-mcp.js`, `npm/platforms/cmsis-dap-mcp-win32-x64/package.json`, `npm/platforms/cmsis-dap-mcp-linux-x64/package.json`, `npm/platforms/cmsis-dap-mcp-darwin-x64/package.json`
- Test: `npm pack --dry-run`（本机）

**Interfaces:**
- Produces: 元包 `cmsis-dap-mcp`（bin 指向启动脚本）；平台包以 `optionalDependencies` 方式按 os/cpu 解析。

- [ ] **Step 1: 创建元包与平台包**

```json
// npm/package.json
{
  "name": "cmsis-dap-mcp",
  "version": "0.1.0",
  "description": "MCP server for CMSIS-DAP debug probes (Cortex-M)",
  "license": "MIT OR Apache-2.0",
  "bin": { "cmsis-dap-mcp": "bin/cmsis-dap-mcp.js" },
  "files": ["bin"],
  "optionalDependencies": {
    "cmsis-dap-mcp-win32-x64": "0.1.0",
    "cmsis-dap-mcp-linux-x64": "0.1.0",
    "cmsis-dap-mcp-darwin-x64": "0.1.0"
  }
}
```

```json
// npm/platforms/cmsis-dap-mcp-win32-x64/package.json
{
  "name": "cmsis-dap-mcp-win32-x64",
  "version": "0.1.0",
  "os": ["win32"],
  "cpu": ["x64"],
  "license": "MIT OR Apache-2.0",
  "bin": { "cmsis-dap-mcp": "bin/cmsis-dap-mcp.exe" },
  "files": ["bin"]
}
```

`npm/bin/cmsis-dap-mcp.js`：根据 `process.platform`/`process.arch` 拼平台包名，`require.resolve` 找到二进制后 `spawn`（stdio 继承）。

- [ ] **Step 2: 验证 npm 包结构**

Run: `cd npm && npm pack --dry-run`
Expected: 输出 tarball 文件列表且包含 `bin/cmsis-dap-mcp.js`。

- [ ] **Step 3: README 内容**

README 包含：简介、三平台二进制下载链接、npm 快速开始（`codex mcp add cmsis-dap-mcp -- npx -y cmsis-dap-mcp`）、原生程序方式、工具表、安全说明、SVD 与 Flash 用法、Linux udev 指引、开发/构建/测试命令、许可证。禁止出现厂商名。

- [ ] **Step 4: 提交**

```bash
git add README.md npm
git commit -m "feat: npm wrapper packages and README"
```

---

### Task 13: GitHub Actions（CI / Release / Pages）

**Files:**
- Create: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/workflows/pages.yml`

**Interfaces:**
- Produces: `ci.yml`（三平台矩阵：fmt、clippy、test、release build）；`release.yml`（`v*` tag：构建三平台产物、上传 Release assets、生成 npm 平台包并 `npm pack`；`npm publish` 仅在 `secrets.NPM_TOKEN` 存在时执行）；`pages.yml`（mdBook 构建部署到 GitHub Pages）。

- [ ] **Step 1: 写 ci.yml**

```yaml
name: CI
on: [push, pull_request]
jobs:
  check:
    strategy:
      fail-fast: false
      matrix:
        os: [windows-latest, ubuntu-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { components: "rustfmt,clippy" }
      - run: cargo fmt --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test
      - run: cargo build --release
```

- [ ] **Step 2: 写 release.yml**

tag 触发；三平台 job 构建 `--release`，把 `cmsis-dap-mcp`/`.exe` 上传到 `softprops/action-gh-release@v2`；额外 job 生成 npm 平台包并 `npm pack`；`npm publish` 步骤仅在 `env.NPM_TOKEN != ''` 时执行。

- [ ] **Step 3: 写 pages.yml**

`on: push: branches: [main]`；步骤：安装 mdBook、`mdbook build docs`、`peaceiris/actions-gh-pages@v4` 发布 `docs/book`。

- [ ] **Step 4: 本地校验 YAML**

Run: `python -c "import yaml,sys; [yaml.safe_load(open(f,encoding='utf-8')) for f in ['.github/workflows/ci.yml','.github/workflows/release.yml','.github/workflows/pages.yml']]"`（若缺 pyyaml 则先安装到用户目录）。
Expected: 无异常。

- [ ] **Step 5: 提交**

```bash
git add .github/workflows
git commit -m "ci: add CI, release and pages workflows"
```

---

### Task 14: mdBook 文档

**Files:**
- Create: `docs/book.toml`, `docs/src/SUMMARY.md`, `docs/src/index.md`, `docs/src/quickstart.md`, `docs/src/tools.md`, `docs/src/security.md`, `docs/src/svd-flash.md`
- Test: `mdbook build`（本机）

**Interfaces:**
- Produces: 文档站点源码；Pages 工作流构建产物 `docs/book`。

- [ ] **Step 1: 安装并初始化 mdBook**

Run: `cargo install mdbook --locked`；`mdbook init docs --title "CMSIS-DAP MCP"`。

- [ ] **Step 2: 编写页面**

`index.md`：项目简介与能力边界（通用 Cortex-M 调试；SVD/Flash 为可选）；`quickstart.md`：npx 与原生程序两种 MCP 配置示例（Codex `config.toml` 示例）；`tools.md`：设计规格第 6 节工具表与参数；`security.md`：三级安全与 `--allow-destructive`；`svd-flash.md`：运行时 SVD/CMSIS-Pack 用法，明确“芯片资料由用户提供，不在仓库内”。

- [ ] **Step 3: 构建验证**

Run: `mdbook build docs`
Expected: `docs/book/index.html` 生成。

- [ ] **Step 4: 提交**

```bash
git add docs
git commit -m "docs: add mdBook site"
```
---

### Task 15: 许可证、厂商内容扫描与发布前自检

**Files:**
- Create: `LICENSE-APACHE`, `LICENSE-MIT`, `scripts/check-no-vendor.ps1`
- Modify: `.gitignore`

**Interfaces:**
- Produces: `scripts/check-no-vendor.ps1`：对仓库（排除自身与构建产物）执行 vendor 关键词扫描，命中即退出码 1。

- [ ] **Step 1: 写许可证与扫描脚本**

`LICENSE-APACHE`/`LICENSE-MIT` 使用标准文本（Apache/MIT 官方模板）。扫描脚本：

```powershell
# scripts/check-no-vendor.ps1
$pattern = 'ALB{0}|ALBS{1}' -f '32','EMI'`n$hits = rg -i $pattern . --glob '!target/**' --glob '!.git/**' --glob '!scripts/check-no-vendor.ps1' 2>$null
if ($LASTEXITCODE -eq 0) {
  Write-Error "vendor-specific content found:`n$hits"
  exit 1
}
Write-Output "no vendor-specific content"
```

- [ ] **Step 2: 运行自检**

Run: `powershell -File scripts/check-no-vendor.ps1`、`git diff --check`、`cargo test`、`cargo build --release`、`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check`
Expected: 全部通过；扫描无输出。

- [ ] **Step 3: 修正规格中的厂商内容（若扫描命中）**

若设计规格或计划出现厂商字样，将其替换为通用描述（如“本地厂商 SDK（仓库外）”“本地 Cortex-M0 验证板”），并重新提交。

- [ ] **Step 4: 提交**

```bash
git add LICENSE-APACHE LICENSE-MIT scripts .gitignore
git commit -m "chore: add licenses and vendor-content scan"
```

---

### Task 16: 本地实机验证与 GitFlow 收尾（不入库内容）

**Files:**
- 不新增仓库文件；验证结果记录在仓库外本地会话笔记。

**Interfaces:**
- Consumes: 设计规格第 18 节所列本机硬件与厂商 SDK（路径保持在仓库外）。

- [ ] **Step 1: 生成本地 target 描述**

在仓库外目录（如 `%USERPROFILE%\cmsis-dap-mcp-verify`）使用厂商 SDK 的 DFP/FLM 经 probe-rs `target-gen` 生成 target YAML；验证命令不写入仓库。

- [ ] **Step 2: 无破坏性验证**

Run: `cargo run --release -- --probe-id <本地序列号> --protocol swd` 配合 MCP 客户端（或测试脚本）依次执行：`list_probes`、`connect`、`get_target_info`、`read_memory`（RAM 与 SVD 已知地址）、`write_memory` 后回读、`halt/resume/step`、断点、SVD 命名读取。
Expected: 全部成功；记录输出。

- [ ] **Step 3: 破坏性验证（用户确认后进行）**

Run: 使用 `--allow-destructive` 与本地 target 描述，对测试地址执行 `erase_flash`/`program_flash`，随后回读校验。
Expected: 擦写与回读一致。

- [ ] **Step 4: GitFlow 合并与发布前检查**

```bash
git switch main
git merge --no-ff develop -m "merge: develop into main for v0.1.0"
git tag v0.1.0
```

运行 Task 15 全部自检；推送前再次确认仓库扫描无厂商内容。

- [ ] **Step 5: 提交与推送（等待用户提供远程与凭据）**

```bash
git remote add origin <GITHUB_REPO_URL>
git push -u origin main develop --tags
```

推送后创建 GitHub Release（经 `gh` 或网页），触发 release.yml 产出三平台资产与 npm 包。

---

## Self-Review Checklist

- [ ] 规格第 6 节全部 24 个工具在 Task 6-11 中均有对应实现。
- [ ] 三级安全在 Task 2（策略）与 Task 6-11（工具调用）中闭环。
- [ ] “日志不写 stdout”在 Task 1 与 Task 11 的 main 中落实。
- [ ] 无厂商内容约束在 Task 15 有强制扫描，且 Task 16 的验证内容不进入仓库。
- [ ] 三平台 CI、Release、Pages 在 Task 13 落实；npm 双入口在 Task 12 落实。
- [ ] GitFlow 合并与本地全量验证在 Task 16 落实。
- [ ] 所有任务均以测试先行（TDD），无占位步骤。