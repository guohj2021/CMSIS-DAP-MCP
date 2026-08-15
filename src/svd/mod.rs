use crate::error::{ErrorCode, McpError};
use std::path::Path;

pub struct SvdSummary {
    pub name: String,
    pub peripherals: usize,
}

pub struct SvdDatabase;

impl SvdDatabase {
    pub fn load(_path: &Path) -> Result<Self, McpError> {
        Err(McpError::new(
            ErrorCode::UnsupportedFeature,
            "SVD parsing lands in a later task",
        ))
    }

    pub fn summary(&self) -> SvdSummary {
        SvdSummary { name: "stub".into(), peripherals: 0 }
    }
}