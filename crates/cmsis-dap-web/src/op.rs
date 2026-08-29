//! Operation model, three-layer state model and server events (frozen v5 §3-§4).

use cmsis_dap_core::error::{ErrorCode, McpError};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Three-layer state model (v5 §3)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerState {
    Ready,
    Busy,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetState {
    Running,
    Halted,
    Resetting,
    Fault,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    None,
    Debug,
    Flash,
    Rtt,
    Evr,
}

/// Combined status broadcast to the UI (and returned by GET /api/status).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatus {
    pub server: ServerState,
    pub target: TargetState,
    pub operation: OperationState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pc: Option<u64>,
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self {
            server: ServerState::Disconnected,
            target: TargetState::Unknown,
            operation: OperationState::None,
            reason: None,
            pc: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Timeout policy (v5 §4)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum TimeoutPolicy {
    /// Ordinary operations with a fixed timeout.
    Default(Duration),
    /// Long-running (flash); `max` is the hard limit, `watchdog` reports
    /// backend I/O stalls via the progress path.
    LongRunning { max: Duration, watchdog: bool },
    /// Cancellable (Halt/Reset/Disconnect may preempt an interruptible op).
    Cancelable,
}

// ---------------------------------------------------------------------------
// Operation model (v5 §4)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    ListProbes,
    TargetInfo,
    Connect,
    Disconnect,
    Run,
    Halt,
    Step,
    Reset,
    RegisterRead,
    RegisterWrite,
    ListRegisters,
    MemoryRead,
    MemoryWrite,
    Breakpoint,
    BreakpointLimits,
    Watchpoint,
    WatchRead,
    PeripheralRead,
    PeripheralWrite,
    RttRead,
    EvrRead,
    Flash,
    Snapshot,
    Status,
    DumpFault,
}

impl OperationKind {
    /// The target must be halted for this operation to be meaningful.
    pub fn requires_halt(self) -> bool {
        // Note: Flash is intentionally excluded — probe-rs' flash loader
        // manages core halt internally (same as the CLI `flash` command).
        matches!(
            self,
            OperationKind::RegisterRead | OperationKind::RegisterWrite | OperationKind::Step
        )
    }

    /// Whether this operation may be interrupted by Halt/Reset/Disconnect.
    pub fn can_interrupt(self) -> bool {
        matches!(
            self,
            OperationKind::Run
                | OperationKind::Halt
                | OperationKind::Reset
                | OperationKind::Disconnect
                | OperationKind::RttRead
                | OperationKind::EvrRead
                | OperationKind::Snapshot
        )
    }

    /// Destructive tier: gated by `--allow-destructive` (flash only in V0.1;
    /// flash software breakpoints are gated at the parameter level).
    pub fn destructive(self) -> bool {
        matches!(self, OperationKind::Flash)
    }

    /// Write tier: permitted once connected, but the UI must confirm.
    pub fn write_tier(self) -> bool {
        matches!(
            self,
            OperationKind::RegisterWrite
                | OperationKind::MemoryWrite
                | OperationKind::PeripheralWrite
        )
    }


    /// Exclusive operations set `OperationState` and reject concurrent ops.
    pub fn exclusive(self) -> bool {
        matches!(
            self,
            OperationKind::Connect
                | OperationKind::Disconnect
                | OperationKind::Reset
                | OperationKind::Flash
                | OperationKind::Snapshot
        )
    }

    pub fn timeout_policy(self) -> TimeoutPolicy {
        match self {
            OperationKind::Connect => TimeoutPolicy::Default(Duration::from_secs(30)),
            OperationKind::Disconnect => TimeoutPolicy::Default(Duration::from_secs(10)),
            OperationKind::Run | OperationKind::Halt | OperationKind::Step => {
                TimeoutPolicy::Cancelable
            }
            OperationKind::Reset => TimeoutPolicy::Cancelable,
            OperationKind::Flash => TimeoutPolicy::LongRunning {
                max: Duration::from_secs(600),
                watchdog: true,
            },
            OperationKind::Snapshot => TimeoutPolicy::Default(Duration::from_secs(30)),
            _ => TimeoutPolicy::Default(Duration::from_secs(2)),
        }
    }

    /// The operation state shown while this operation runs (or is exclusive).
    pub fn operation_state(self) -> OperationState {
        match self {
            OperationKind::Flash => OperationState::Flash,
            OperationKind::RttRead => OperationState::Rtt,
            OperationKind::EvrRead => OperationState::Evr,
            _ => OperationState::Debug,
        }
    }
}

// ---------------------------------------------------------------------------
// Operation request / result
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Operation {
    pub id: u64,
    pub kind: OperationKind,
    /// Serialized JSON parameters (mirrors MCP/remote param shapes).
    pub params: serde_json::Value,
    pub reply: tokio::sync::oneshot::Sender<Result<serde_json::Value, WebError>>,
}

impl Operation {
    pub fn new(
        id: u64,
        kind: OperationKind,
        params: serde_json::Value,
    ) -> (Self, tokio::sync::oneshot::Receiver<Result<serde_json::Value, WebError>>) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        (
            Self {
                id,
                kind,
                params,
                reply: tx,
            },
            rx,
        )
    }
}

// ---------------------------------------------------------------------------
// Web error model (v5 §11)
// ---------------------------------------------------------------------------

/// HTTP error code -> status mapping lives in `api.rs`.
#[derive(Debug, Clone, thiserror::Error)]
pub enum WebError {
    #[error("{0}")]
    Mcp(#[from] McpError),
    #[error("session busy with another operation: {0}")]
    SessionBusy(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotConnected(String),
    #[error("{0}")]
    DestructiveDisabled(String),
    #[error("firmware image exceeds target flash region: {0}")]
    RegionOverflow(String),
    #[error("probe lost: {0}")]
    ProbeLost(String),
    #[error("{0}")]
    Internal(String),
}

impl WebError {
    pub fn code(&self) -> &'static str {
        match self {
            WebError::Mcp(e) => match e.code {
                ErrorCode::NotConnected => "not_connected",
                ErrorCode::DestructiveDisabled => "destructive_disabled",
                ErrorCode::ProbeNotFound => "probe_not_found",
                ErrorCode::InvalidArgument => "invalid_argument",
                ErrorCode::UnsupportedFeature => "unsupported_feature",
                _ => "error",
            },
            WebError::SessionBusy(_) => "session_busy",
            WebError::InvalidArgument(_) => "invalid_argument",
            WebError::NotConnected(_) => "not_connected",
            WebError::DestructiveDisabled(_) => "destructive_disabled",
            WebError::RegionOverflow(_) => "region_overflow",
            WebError::ProbeLost(_) => "probe_lost",
            WebError::Internal(_) => "internal_error",
        }
    }

    pub fn http_status(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            WebError::SessionBusy(_) | WebError::NotConnected(_) => StatusCode::CONFLICT,
            WebError::DestructiveDisabled(_) => StatusCode::FORBIDDEN,
            WebError::Mcp(e) => match e.code {
                ErrorCode::ProbeNotFound => StatusCode::NOT_FOUND,
                ErrorCode::InvalidArgument => StatusCode::BAD_REQUEST,
                ErrorCode::NotConnected => StatusCode::CONFLICT,
                ErrorCode::DestructiveDisabled => StatusCode::FORBIDDEN,
                _ => StatusCode::BAD_REQUEST,
            },
            WebError::InvalidArgument(_) => StatusCode::BAD_REQUEST,
            WebError::RegionOverflow(_) => StatusCode::UNPROCESSABLE_ENTITY,
            WebError::ProbeLost(_) => StatusCode::CONFLICT,
            WebError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "error": { "code": self.code(), "message": self.to_string() } })
    }
}

// ---------------------------------------------------------------------------
// Server events (v5 §11 WS)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum ServerEvent {
    Ready {
        status: SessionStatus,
    },
    TargetStateChanged {
        #[serde(flatten)]
        status: SessionStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        operation_id: Option<u64>,
    },
    ProbeLost {
        message: String,
    },
    Error {
        code: String,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        operation_id: Option<u64>,
    },
    #[serde(rename_all = "snake_case")]
    FlashProgress {
        operation_id: u64,
        phase: String,
        bytes_done: u64,
        bytes_total: u64,
        current_address: u64,
    },
    #[serde(rename_all = "snake_case")]
    FlashComplete {
        operation_id: u64,
        ok: bool,
        bytes: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    Console {
        level: String,
        text: String,
    },
}

impl ServerEvent {
    pub fn event_name(&self) -> &'static str {
        match self {
            ServerEvent::Ready { .. } => "ready",
            ServerEvent::TargetStateChanged { .. } => "target_state_changed",
            ServerEvent::ProbeLost { .. } => "probe_lost",
            ServerEvent::Error { .. } => "error",
            ServerEvent::FlashProgress { .. } => "flash_progress",
            ServerEvent::FlashComplete { .. } => "flash_complete",
            ServerEvent::Console { .. } => "console",
        }
    }
}


