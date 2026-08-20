use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    ProbeNotFound,
    ConnectFailed,
    NotConnected,
    ProtocolError,
    Timeout,
    MemoryFault,
    SvdNotLoaded,
    FileError,
    UnsupportedFeature,
    DestructiveDisabled,
    InvalidArgument,
    InternalError,
    /// A runtime configuration value was missing, malformed, or rejected.
    ConfigError,
}

#[derive(Debug, Clone, Error)]
#[error("{code:?}: {message}")]
pub struct McpError {
    pub code: ErrorCode,
    pub message: String,
}

impl McpError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
