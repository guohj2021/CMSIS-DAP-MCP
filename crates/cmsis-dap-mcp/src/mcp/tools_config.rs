use schemars::JsonSchema;
use serde::Deserialize;

/// No parameters: returns the current runtime config.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetConfigParams {}

/// Partial update of the runtime config. Omit any field to keep its current
/// value. Invalid values are rejected as a whole (nothing changes).
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct UpdateConfigParams {
    /// Enable (`true`) or disable (`false`) destructive tools: erase_flash,
    /// program_flash, and destructive script commands (erase, loadbin,
    /// loadfile, flash write_image).
    pub allow_destructive: Option<bool>,
    /// Start or move the remote JSON-RPC TCP server to this port (1-65535).
    /// The server runs alongside MCP on 127.0.0.1.
    pub tcp_port: Option<u16>,
    /// Start the GDB server on this port (1-65535). A running GDB server
    /// cannot be moved at runtime; restart the server to change its port.
    pub gdb_port: Option<u16>,
}

/// No parameters: re-applies the `--config-file` supplied at startup.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ReloadConfigParams {}
