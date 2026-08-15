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
        Self {
            backend,
            connected: None,
            svd: None,
        }
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

    pub fn backend(&mut self) -> &mut dyn Backend {
        self.backend.as_mut()
    }

    pub fn load_svd(&mut self, path: &Path) -> Result<SvdSummary, McpError> {
        let db = SvdDatabase::load(path)?;
        let summary = db.summary();
        self.svd = Some(db);
        Ok(summary)
    }

    pub fn svd(&self) -> Result<&SvdDatabase, McpError> {
        self.svd.as_ref().ok_or_else(|| {
            McpError::new(
                ErrorCode::SvdNotLoaded,
                "load an SVD file with load_svd first",
            )
        })
    }

    pub fn target_info(&self) -> Option<&TargetInfo> {
        self.connected.as_ref()
    }
}
