use crate::error::{ErrorCode, McpError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    ReadOnly,
    Write,
    Destructive,
}

#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    pub allow_destructive: bool,
}

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
