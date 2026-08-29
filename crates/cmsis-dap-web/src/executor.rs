//! DebugExecutor: the single hardware I/O owner (frozen v5 §4, §6).
//!
//! A dedicated OS thread owns the [`SessionManager`]. Every component (REST
//! handlers, schedulers, flash) submits an [`Operation`] over a channel and
//! awaits a one-shot reply; no other code touches the probe.
//!
//! The executor also owns the three-layer state model, the destructive gate
//! and probe-lost handling (session-wide fatal event, v5 §6).

use crate::op::{
    Operation, OperationKind, OperationState, ServerEvent, ServerState, SessionStatus, TargetState,
    WebError,
};
use cmsis_dap_core::backend::FlashPhase;
use cmsis_dap_core::backend::{
    AccessWidth, ConnectOptions, CoreRegister, ImageFileFormat, Protocol, ResetMode, WatchAccess,
};
use cmsis_dap_core::error::ErrorCode;
use cmsis_dap_core::session::SessionManager;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::broadcast;

/// Clonable handle to the executor, used by API handlers and schedulers.
#[derive(Clone)]
pub struct ExecutorHandle {
    op_tx: std::sync::mpsc::Sender<Operation>,
    status: Arc<RwLock<SessionStatus>>,
    events: broadcast::Sender<ServerEvent>,
    next_id: Arc<AtomicU64>,
    allow_destructive: bool,
    shutdown: Arc<AtomicBool>,
}

impl ExecutorHandle {
    /// Submit an operation and wait for its result.
    pub fn call(&self, kind: OperationKind, params: Value) -> Result<Value, WebError> {
        if self.shutdown.load(Ordering::SeqCst) {
            return Err(WebError::Internal("executor is shutting down".into()));
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (op, rx) = Operation::new(id, kind, params);
        self.op_tx
            .send(op)
            .map_err(|_| WebError::Internal("executor channel closed".into()))?;
        rx.blocking_recv()
            .map_err(|_| WebError::Internal("executor stopped before replying".into()))?
    }

    /// Submit an operation and await its result from an async context.
    pub async fn call_async(&self, kind: OperationKind, params: Value) -> Result<Value, WebError> {
        if self.shutdown.load(Ordering::SeqCst) {
            return Err(WebError::Internal("executor is shutting down".into()));
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (op, rx) = Operation::new(id, kind, params);
        self.op_tx
            .send(op)
            .map_err(|_| WebError::Internal("executor channel closed".into()))?;
        rx.await
            .map_err(|_| WebError::Internal("executor stopped before replying".into()))?
    }

    /// Submit an operation without waiting (fire-and-forget, e.g. events).
    pub fn fire(&self, kind: OperationKind, params: Value) -> Result<u64, WebError> {
        if self.shutdown.load(Ordering::SeqCst) {
            return Err(WebError::Internal("executor is shutting down".into()));
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (op, rx) = Operation::new(id, kind, params);
        // Drop the receiver: the executor still executes the op.
        drop(rx);
        self.op_tx
            .send(op)
            .map_err(|_| WebError::Internal("executor channel closed".into()))?;
        Ok(id)
    }

    pub fn status(&self) -> SessionStatus {
        self.status.read().unwrap().clone()
    }

    pub fn events(&self) -> broadcast::Sender<ServerEvent> {
        self.events.clone()
    }

    pub fn allow_destructive(&self) -> bool {
        self.allow_destructive
    }

    pub fn set_status(&self, status: SessionStatus) {
        *self.status.write().unwrap() = status;
    }
}

/// Run state of the executor loop.
struct Active {
    operation: OperationState,
    exclusive: bool,
}

pub struct ExecutorConfig {
    pub allow_destructive: bool,
    pub flash_timeout: Duration,
}

/// Start the executor on a dedicated thread. Returns a handle.
pub fn spawn(session: SessionManager, config: ExecutorConfig) -> ExecutorHandle {
    let (op_tx, op_rx) = std::sync::mpsc::channel::<Operation>();
    let (events, _) = broadcast::channel(256);
    let handle = ExecutorHandle {
        op_tx,
        status: Arc::new(RwLock::new(SessionStatus::default())),
        events,
        next_id: Arc::new(AtomicU64::new(1)),
        allow_destructive: config.allow_destructive,
        shutdown: Arc::new(AtomicBool::new(false)),
    };
    let runner = ExecutorRunner {
        session: std::sync::Mutex::new(session),
        status: handle.status.clone(),
        events: handle.events.clone(),
        allow_destructive: config.allow_destructive,
        flash_timeout: config.flash_timeout,
        shutdown: handle.shutdown.clone(),
    };
    std::thread::Builder::new()
        .name("cmsis-dap-web-executor".into())
        .spawn(move || runner.run(op_rx))
        .expect("failed to spawn debug executor thread");
    handle
}

struct ExecutorRunner {
    session: std::sync::Mutex<SessionManager>,
    status: Arc<RwLock<SessionStatus>>,
    events: broadcast::Sender<ServerEvent>,
    allow_destructive: bool,
    #[allow(dead_code)] // used by the flash pipeline (P3)
    flash_timeout: Duration,
    shutdown: Arc<AtomicBool>,
}

impl ExecutorRunner {
    fn run(mut self, rx: std::sync::mpsc::Receiver<Operation>) {
        let mut active: Option<Active> = None;
        loop {
            match rx.recv_timeout(Duration::from_millis(250)) {
                Ok(op) => {
                    if let Err(e) = self.execute(&mut active, op) {
                        match &e {
                            WebError::SessionBusy(_) => {
                                tracing::debug!("op rejected (session busy): {e}");
                            }
                            _ => tracing::warn!("executor op failed: {e}"),
                        }
                        if matches!(e, WebError::ProbeLost(_)) {
                            self.invalidate_session(&e.to_string());
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if self.shutdown.load(Ordering::SeqCst) {
                        break;
                    }
                }
            }
        }
    }

    /// Gate + execute a single operation.
    fn execute(&mut self, active: &mut Option<Active>, op: Operation) -> Result<(), WebError> {
        // Gate on state.
        let status = self.status.read().unwrap().clone();
        if let Err(e) = self.gate(&op, &status, active) {
            let _ = op.reply.send(Err(e.clone()));
            return Err(e);
        }

        // Destructive gate.
        if op.kind.destructive() && !self.allow_destructive {
            let err = WebError::DestructiveDisabled(format!(
                "{} is disabled (start with --allow-destructive)",
                format!("{:?}", op.kind).to_lowercase()
            ));
            let _ = op.reply.send(Err(err.clone()));
            return Err(err);
        }

        // Mark active.
        let new_active = Active {
            operation: op.kind.operation_state(),
            exclusive: op.kind.exclusive(),
        };
        let prev = active.replace(new_active);
        let mut status = self.status.read().unwrap().clone();
        status.server = ServerState::Busy;
        status.operation = op.kind.operation_state();
        if op.kind == OperationKind::Reset {
            status.target = TargetState::Resetting;
        }
        *self.status.write().unwrap() = status.clone();
        self.emit(ServerEvent::TargetStateChanged {
            status,
            operation_id: Some(op.id),
        });

        let result = self.dispatch(&op);

        // Restore state after the op. The target state set by the dispatch
        // (e.g. Running/Halted after a reset) is preserved.
        let mut status = self.status.read().unwrap().clone();
        status.operation = prev
            .as_ref()
            .map(|p| p.operation)
            .unwrap_or(OperationState::None);
        status.server = match status.server {
            ServerState::Disconnected => ServerState::Disconnected,
            _ => ServerState::Ready,
        };
        *self.status.write().unwrap() = status.clone();
        self.emit(ServerEvent::TargetStateChanged {
            status,
            operation_id: Some(op.id),
        });
        *active = prev;

        // Send reply; on probe-loss, escalate.
        match result {
            Ok(value) => {
                let _ = op.reply.send(Ok(value));
                Ok(())
            }
            Err(e) => {
                let _ = op.reply.send(Err(e.clone()));
                if is_probe_error(&e) {
                    return Err(WebError::ProbeLost(e.to_string()));
                }
                Ok(())
            }
        }
    }

    /// SessionBusy / state gating (v5 §4).
    fn gate(
        &self,
        op: &Operation,
        status: &SessionStatus,
        active: &Option<Active>,
    ) -> Result<(), WebError> {
        // Disconnected: only list/connect allowed.
        if status.server == ServerState::Disconnected
            && !matches!(op.kind, OperationKind::ListProbes | OperationKind::Connect)
        {
            return Err(WebError::NotConnected(
                "not connected; call connect first".into(),
            ));
        }
        // Exclusive op already running.
        if let Some(a) = active {
            if a.exclusive && op.kind != OperationKind::Disconnect {
                return Err(WebError::SessionBusy(format!(
                    "target is busy with {:?}",
                    a.operation
                )));
            }
        }
        // A second exclusive op must not queue behind a non-exclusive one.
        if op.kind.exclusive() && active.is_some() {
            return Err(WebError::SessionBusy("target is busy".into()));
        }
        // Register/step/flash require a halted target.
        if op.kind.requires_halt() && status.target != TargetState::Halted {
            return Err(WebError::InvalidArgument(
                "operation requires a halted target".into(),
            ));
        }
        Ok(())
    }

    fn dispatch(&mut self, op: &Operation) -> Result<Value, WebError> {
        let mut session = self
            .session
            .lock()
            .map_err(|_| WebError::Internal("session lock poisoned".into()))?;
        let p = &op.params;

        // All ops except listing/connecting/disconnecting require a session.
        if !matches!(
            op.kind,
            OperationKind::ListProbes | OperationKind::Connect | OperationKind::Disconnect
        ) {
            session.ensure_connected()?;
        }

        match op.kind {
            OperationKind::ListProbes => {
                let backend = session.backend();
                Ok(json!({ "probes": backend.list_probes()? }))
            }
            OperationKind::TargetInfo => {
                session.ensure_connected()?;
                let info = session
                    .target_info()
                    .ok_or_else(|| WebError::NotConnected("no target".into()))?
                    .clone();
                Ok(json!({ "target": info }))
            }
            OperationKind::Connect => {
                let opts = parse_connect_options(p)?;
                let info = session.connect(&opts)?;
                let mut status = self.status.write().unwrap();
                status.server = ServerState::Ready;
                status.target = TargetState::Unknown;
                status.operation = OperationState::None;
                drop(status);
                Ok(json!({ "target": info }))
            }
            OperationKind::Disconnect => {
                session.disconnect()?;
                let mut status = self.status.write().unwrap();
                *status = SessionStatus {
                    server: ServerState::Disconnected,
                    ..Default::default()
                };
                drop(status);
                Ok(json!({ "disconnected": true }))
            }
            OperationKind::Status => {
                let backend = session.backend();
                let core = backend.get_core_status()?;
                // Keep the executor's TargetState in sync with reality
                // (e.g. a hardware breakpoint halt while "running").
                let mut status = self.status.write().unwrap();
                status.target = match core.state.as_str() {
                    "running" | "sleeping" => TargetState::Running,
                    "halted" => TargetState::Halted,
                    _ => status.target,
                };
                status.reason = core.halt_reason.clone();
                status.pc = core.pc;
                drop(status);
                Ok(json!({ "status": core }))
            }
            OperationKind::Run => {
                let backend = session.backend();
                backend.resume()?;
                let mut status = self.status.write().unwrap();
                status.target = TargetState::Running;
                drop(status);
                Ok(json!({ "running": true }))
            }
            OperationKind::Halt => {
                let backend = session.backend();
                backend.halt()?;
                let mut status = self.status.write().unwrap();
                status.target = TargetState::Halted;
                drop(status);
                Ok(json!({ "halted": true }))
            }
            OperationKind::Step => {
                let backend = session.backend();
                backend.step()?;
                Ok(json!({ "stepped": true }))
            }
            OperationKind::Reset => {
                let mode = match p.get("mode").and_then(|v| v.as_str()).unwrap_or("run") {
                    "run" => ResetMode::Run,
                    "halt" => ResetMode::Halt,
                    other => {
                        return Err(WebError::InvalidArgument(format!(
                            "reset mode must be run or halt, got {other}"
                        )))
                    }
                };
                let backend = session.backend();
                backend.reset(mode)?;
                let mut status = self.status.write().unwrap();
                status.target = if mode == ResetMode::Run {
                    TargetState::Running
                } else {
                    TargetState::Halted
                };
                drop(status);
                Ok(
                    json!({ "reset": true, "mode": if mode == ResetMode::Run { "run" } else { "halt" } }),
                )
            }
            OperationKind::ListRegisters => {
                let backend = session.backend();
                let names = backend.list_core_registers()?;
                let mut registers = Vec::new();
                for name in &names {
                    if let Ok(value) = backend.read_core_register(&CoreRegister::Name(name.clone()))
                    {
                        registers.push(json!({ "name": name, "value": value }));
                    }
                }
                Ok(json!({ "registers": registers }))
            }
            OperationKind::RegisterRead => {
                let backend = session.backend();
                let name = param_str(p, "name")?;
                let value = backend.read_core_register(&CoreRegister::Name(name.clone()))?;
                Ok(json!({ "register": name, "value": value }))
            }
            OperationKind::RegisterWrite => {
                let backend = session.backend();
                let name = param_str(p, "name")?;
                let value = param_u64(p, "value")?;
                backend.write_core_register(&CoreRegister::Name(name), value)?;
                Ok(json!({ "written": true }))
            }
            OperationKind::MemoryRead => {
                let backend = session.backend();
                let address = param_u64(p, "address")?;
                let width = param_width(p)?;
                let count = p.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                let values = backend.read_memory(address, width, count)?;
                let bytes = values_to_bytes(&values, width);
                Ok(json!({ "address": address, "bytes": bytes }))
            }
            OperationKind::MemoryWrite => {
                let backend = session.backend();
                let address = param_u64(p, "address")?;
                let width = param_width(p)?;
                let values: Vec<u64> = p
                    .get("values")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_u64()).collect())
                    .ok_or_else(|| WebError::InvalidArgument("missing values".into()))?;
                backend.write_memory(address, width, &values)?;
                Ok(json!({ "written": true }))
            }
            OperationKind::Breakpoint => {
                let backend = session.backend();
                let action = param_str(p, "action").unwrap_or("set".into());
                match action.as_str() {
                    "set" => {
                        let address = param_u64(p, "address")?;
                        backend.set_breakpoint(address)?;
                        Ok(json!({ "set": address }))
                    }
                    "set_flash" => {
                        if !self.allow_destructive {
                            return Err(WebError::DestructiveDisabled(
                                "flash software breakpoints require --allow-destructive".into(),
                            ));
                        }
                        let address = param_u64(p, "address")?;
                        backend.set_flash_breakpoint(address)?;
                        Ok(json!({ "set_flash": address }))
                    }
                    "clear" => {
                        backend.clear_breakpoints()?;
                        Ok(json!({ "cleared": true }))
                    }
                    "clear_flash" => {
                        backend.clear_flash_breakpoints()?;
                        Ok(json!({ "cleared_flash": true }))
                    }
                    "list" => Ok(json!({ "breakpoints": backend.list_breakpoints()? })),
                    "list_flash" => Ok(json!({ "breakpoints": backend.list_flash_breakpoints()? })),
                    other => Err(WebError::InvalidArgument(format!(
                        "unknown breakpoint action {other}"
                    ))),
                }
            }
            OperationKind::BreakpointLimits => {
                let backend = session.backend();
                let limits = backend.hw_breakpoint_limits()?;
                match limits {
                    Some((used, total)) => Ok(json!({ "used": used, "total": total })),
                    None => Ok(json!({ "used": null, "total": null })),
                }
            }
            OperationKind::Watchpoint => {
                let backend = session.backend();
                let action = param_str(p, "action").unwrap_or("set".into());
                match action.as_str() {
                    "set" => {
                        let address = param_u64(p, "address")?;
                        let access = match p.get("access").and_then(|v| v.as_str()).unwrap_or("rw")
                        {
                            "read" => WatchAccess::Read,
                            "write" => WatchAccess::Write,
                            "rw" => WatchAccess::ReadWrite,
                            other => {
                                return Err(WebError::InvalidArgument(format!(
                                    "watchpoint access must be read/write/rw, got {other}"
                                )))
                            }
                        };
                        backend.set_watchpoint(address, access)?;
                        Ok(json!({ "set": address }))
                    }
                    "clear" => {
                        backend.clear_watchpoints()?;
                        Ok(json!({ "cleared": true }))
                    }
                    "list" => Ok(json!({ "watchpoints": backend.list_watchpoints()? })),
                    other => Err(WebError::InvalidArgument(format!(
                        "unknown watchpoint action {other}"
                    ))),
                }
            }
            OperationKind::WatchRead => {
                let backend = session.backend();
                let address = param_u64(p, "address")?;
                let width = param_width(p)?;
                let values = backend.read_memory(address, width, 1)?;
                Ok(json!({ "value": values.first().copied().unwrap_or(0) }))
            }
            OperationKind::PeripheralRead => {
                let backend = session.backend();
                let address = param_u64(p, "address")?;
                let values = backend.read_memory(address, AccessWidth::U32, 1)?;
                let value = values.first().copied().unwrap_or(0) as u32;
                Ok(json!({ "value": value }))
            }
            OperationKind::PeripheralWrite => {
                let backend = session.backend();
                let address = param_u64(p, "address")?;
                let value = param_u64(p, "value")?;
                backend.write_memory(address, AccessWidth::U32, &[value])?;
                Ok(json!({ "written": true }))
            }
            OperationKind::RttAttach => {
                let backend = session.backend();
                let address = p.get("address").and_then(|v| v.as_u64());
                let channels = backend.attach_rtt(address)?;
                Ok(json!({ "channels": channels }))
            }
            OperationKind::RttRead => {
                let backend = session.backend();
                let channels: Vec<usize> = p
                    .get("channels")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_u64())
                            .map(|x| x as usize)
                            .collect()
                    })
                    .unwrap_or_default();
                let max_bytes =
                    p.get("max_bytes").and_then(|v| v.as_u64()).unwrap_or(4096) as usize;
                let data = backend.read_rtt(&channels, max_bytes)?;
                Ok(serde_json::to_value(data).map_err(|e| WebError::Internal(e.to_string()))?)
            }
            OperationKind::RttDetach => {
                let backend = session.backend();
                backend.detach_rtt()?;
                Ok(json!({ "detached": true }))
            }
            OperationKind::EvrAttach => {
                let backend = session.backend();
                let info_address = param_u64(p, "info_address")?;
                let status = backend.attach_evr(info_address)?;
                Ok(serde_json::to_value(status).map_err(|e| WebError::Internal(e.to_string()))?)
            }
            OperationKind::EvrRead => {
                let backend = session.backend();
                let events = backend.read_evr()?;
                Ok(serde_json::to_value(events).map_err(|e| WebError::Internal(e.to_string()))?)
            }
            OperationKind::EvrDetach => {
                let backend = session.backend();
                backend.detach_evr()?;
                Ok(json!({ "detached": true }))
            }
            OperationKind::Snapshot => {
                let backend = session.backend();
                let addresses: Vec<u64> = p
                    .get("addresses")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_u64()).collect())
                    .unwrap_or_default();
                let stack_words =
                    p.get("stack_words").and_then(|v| v.as_u64()).unwrap_or(16) as usize;
                let restore = p.get("restore").and_then(|v| v.as_bool()).unwrap_or(true);
                let dump = backend.dump_cpu_state(&addresses, stack_words, restore)?;
                Ok(serde_json::to_value(dump).map_err(|e| WebError::Internal(e.to_string()))?)
            }
            OperationKind::DumpFault => {
                let backend = session.backend();
                let dump = backend.dump_cpu_state(&[], 0, false)?;
                Ok(json!({ "fault": dump.fault }))
            }
            OperationKind::Flash => {
                if !self.allow_destructive {
                    return Err(WebError::DestructiveDisabled(
                        "flash requires --allow-destructive".into(),
                    ));
                }
                let action = param_str(p, "action").unwrap_or("program".into());
                let op_id = op.id;
                let events = self.events.clone();
                let events_done = events.clone();
                let base = p.get("address").and_then(|v| v.as_u64()).unwrap_or(0);
                let mut cb = move |phase: FlashPhase, done: u64, total: u64, _addr: u64| {
                    let phase_name = serde_json::to_string(&phase)
                        .map(|s| s.trim_matches('"').to_string())
                        .unwrap_or_else(|_| "unknown".into());
                    let _ = events.send(ServerEvent::FlashProgress {
                        operation_id: op_id,
                        phase: phase_name,
                        bytes_done: done,
                        bytes_total: total,
                        current_address: base + done,
                    });
                };
                match action.as_str() {
                    "erase" => {
                        let address = param_u64(p, "address")?;
                        let size = param_u64(p, "size")?;
                        let backend = session.backend();
                        backend.erase_flash_with_progress(address, size, &mut cb)?;
                        Ok(json!({ "erased": true, "address": address, "size": size }))
                    }
                    "program" => {
                        let path_str = param_str(p, "path")?;
                        let format = parse_image_format(param_str(p, "format")?.as_str())?;
                        let address = param_u64(p, "address")?;
                        let verify = p.get("verify").and_then(|v| v.as_bool()).unwrap_or(true);
                        let mode = param_str(p, "mode").unwrap_or("stay_halted".into());
                        let backend = session.backend();
                        let bytes = backend.program_file_with_progress(
                            std::path::Path::new(&path_str),
                            format,
                            address,
                            verify,
                            &mut cb,
                        )?;
                        match mode.as_str() {
                            "reset" | "reset_run" => backend.reset(ResetMode::Run)?,
                            "reset_halt" => backend.reset(ResetMode::Halt)?,
                            _ => {}
                        }
                        let _ = events_done.send(ServerEvent::FlashComplete {
                            operation_id: op_id,
                            ok: true,
                            bytes,
                            message: None,
                        });
                        Ok(
                            json!({ "programmed": true, "bytes": bytes, "verify": verify, "mode": mode }),
                        )
                    }
                    other => Err(WebError::InvalidArgument(format!(
                        "unknown flash action {other}"
                    ))),
                }
            }
        }
    }
    /// Probe-lost: session-wide fatal event (v5 §6). Immediate, no grace.
    fn invalidate_session(&mut self, message: &str) {
        tracing::error!("probe lost, invalidating session: {message}");
        let mut session = match self.session.lock() {
            Ok(s) => s,
            Err(_) => return,
        };
        let _ = session.disconnect();
        drop(session);
        let status = SessionStatus {
            server: ServerState::Disconnected,
            target: TargetState::Unknown,
            operation: OperationState::None,
            reason: Some("probe lost".into()),
            pc: None,
        };
        *self.status.write().unwrap() = status.clone();
        self.emit(ServerEvent::TargetStateChanged {
            status: status.clone(),
            operation_id: None,
        });
        self.emit(ServerEvent::ProbeLost {
            message: message.to_string(),
        });
    }

    fn emit(&self, event: ServerEvent) {
        let _ = self.events.send(event);
    }
}

/// Decide whether a backend error means the probe disappeared (USB-level).
///
/// Only genuine probe/USB failures invalidate the session. Expected target
/// errors (e.g. "RTT control block not found") must NOT be treated as probe
/// loss — they are normal runtime errors.
fn is_probe_error(e: &WebError) -> bool {
    match e {
        WebError::ProbeLost(_) => true,
        WebError::Mcp(m) => match m.code {
            ErrorCode::ProbeNotFound | ErrorCode::ConnectFailed => true,
            ErrorCode::ProtocolError => {
                let msg = m.message.to_ascii_lowercase();
                msg.contains("debug probe")
                    || msg.contains("command id in response")
                    || msg.contains("cmsis-dap command")
            }
            _ => false,
        },
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Param helpers
// ---------------------------------------------------------------------------

fn parse_image_format(s: &str) -> Result<ImageFileFormat, WebError> {
    ImageFileFormat::parse(s).ok_or_else(|| {
        WebError::InvalidArgument(format!("file format must be elf/axf/bin/hex, got {s}"))
    })
}

fn param_str(p: &Value, name: &str) -> Result<String, WebError> {
    p.get(name)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| WebError::InvalidArgument(format!("missing {name}")))
}

fn param_u64(p: &Value, name: &str) -> Result<u64, WebError> {
    p.get(name)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| WebError::InvalidArgument(format!("missing {name}")))
}

fn param_width(p: &Value) -> Result<AccessWidth, WebError> {
    parse_width(p.get("width").and_then(|v| v.as_str()).unwrap_or("u32"))
}

pub fn parse_width(s: &str) -> Result<AccessWidth, WebError> {
    match s {
        "u8" => Ok(AccessWidth::U8),
        "u16" => Ok(AccessWidth::U16),
        "u32" => Ok(AccessWidth::U32),
        "u64" => Ok(AccessWidth::U64),
        other => Err(WebError::InvalidArgument(format!(
            "width must be u8/u16/u32/u64, got {other}"
        ))),
    }
}

fn parse_protocol(s: &str) -> Result<Protocol, WebError> {
    match s {
        "swd" => Ok(Protocol::Swd),
        "jtag" => Ok(Protocol::Jtag),
        other => Err(WebError::InvalidArgument(format!(
            "protocol must be swd or jtag, got {other}"
        ))),
    }
}

pub fn parse_connect_options(p: &Value) -> Result<ConnectOptions, WebError> {
    Ok(ConnectOptions {
        probe_id: p.get("probe_id").and_then(|v| v.as_str()).map(String::from),
        protocol: parse_protocol(p.get("protocol").and_then(|v| v.as_str()).unwrap_or("swd"))?,
        speed_khz: p
            .get("speed_khz")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        target: p.get("target").and_then(|v| v.as_str()).map(String::from),
        under_reset: p
            .get("under_reset")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        core_index: p
            .get("core_index")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize),
    })
}

fn values_to_bytes(values: &[u64], width: AccessWidth) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * width.byte_size() as usize);
    for v in values {
        let bytes = v.to_le_bytes();
        out.extend_from_slice(&bytes[..width.byte_size() as usize]);
    }
    out
}

// Keep unused import warnings silent for PathBuf until P3 uses it.
#[allow(dead_code)]
fn _unused(p: PathBuf) -> PathBuf {
    p
}
