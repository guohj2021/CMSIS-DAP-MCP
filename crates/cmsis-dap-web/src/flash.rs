//! Firmware image model + flash pipeline helpers (frozen v5 §7).
//!
//! ELF/AXF = debug information; BIN/HEX = firmware image. Uploading an image
//! never flashes it; only `POST /api/flash/program` does, after region
//! validation.

use crate::executor::ExecutorHandle;
use crate::op::{OperationKind, WebError};
use cmsis_dap_core::hex::HexSegment;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FirmwareFormat {
    Bin,
    Hex,
}

impl FirmwareFormat {
    pub fn parse_str(s: &str) -> Result<Self, WebError> {
        match s.to_ascii_lowercase().as_str() {
            "bin" | "binary" => Ok(FirmwareFormat::Bin),
            "hex" | "ihex" | "intelhex" => Ok(FirmwareFormat::Hex),
            other => Err(WebError::InvalidArgument(format!(
                "firmware format must be bin or hex, got {other}"
            ))),
        }
    }

    pub fn from_extension(path: &Path) -> Result<Self, WebError> {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        Self::parse_str(&ext)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            FirmwareFormat::Bin => "bin",
            FirmwareFormat::Hex => "hex",
        }
    }
}

/// A firmware image loaded for programming.
#[derive(Debug, Clone, Serialize)]
pub struct FirmwareImage {
    pub format: FirmwareFormat,
    pub segments: Vec<HexSegment>,
    pub total_size: u64,
    pub address_range: Option<(u64, u64)>,
}

impl FirmwareImage {
    /// Build an image from an uploaded file.
    pub fn analyze(
        path: &Path,
        format: FirmwareFormat,
        bin_address: Option<u64>,
    ) -> Result<Self, WebError> {
        match format {
            FirmwareFormat::Bin => {
                let data = std::fs::read(path)
                    .map_err(|e| WebError::Internal(format!("failed to read upload: {e}")))?;
                let address = bin_address
                    .ok_or_else(|| WebError::InvalidArgument("BIN requires an address".into()))?;
                if data.is_empty() {
                    return Err(WebError::InvalidArgument("BIN file is empty".into()));
                }
                let seg = HexSegment {
                    start: address,
                    end: address + data.len() as u64,
                    data,
                };
                let total = seg.size();
                let seg_end = seg.end;
                Ok(Self {
                    format,
                    segments: vec![seg],
                    total_size: total,
                    address_range: Some((address, seg_end)),
                })
            }
            FirmwareFormat::Hex => {
                let text = std::fs::read_to_string(path)
                    .map_err(|e| WebError::Internal(format!("failed to read upload: {e}")))?;
                let segments =
                    cmsis_dap_core::hex::parse_ihex(&text).map_err(WebError::InvalidArgument)?;
                if segments.is_empty() {
                    return Err(WebError::InvalidArgument(
                        "HEX file contains no data".into(),
                    ));
                }
                let total: u64 = segments.iter().map(|s| s.size()).sum();
                let start = segments.first().map(|s| s.start).unwrap_or(0);
                let end = segments.last().map(|s| s.end).unwrap_or(start);
                Ok(Self {
                    format,
                    segments,
                    total_size: total,
                    address_range: Some((start, end)),
                })
            }
        }
    }

    /// Region validation (v5 §7): every segment must lie inside a target NVM
    /// region. Fails fast before any erase/program.
    pub fn validate(&self, nvm_regions: &[(u64, u64)]) -> Result<(), WebError> {
        for seg in &self.segments {
            let inside = nvm_regions
                .iter()
                .any(|(start, end)| seg.start >= *start && seg.end <= *end);
            if !inside {
                return Err(WebError::RegionOverflow(format!(
                    "segment 0x{:x}..0x{:x} is not inside any flash region",
                    seg.start, seg.end
                )));
            }
        }
        Ok(())
    }
}

/// Program an uploaded image through the executor (single I/O owner).
pub async fn program_image(
    executor: &ExecutorHandle,
    _file_id: &str,
    address: Option<u64>,
    verify: bool,
    mode: &str,
    format: FirmwareFormat,
    path: &Path,
) -> Result<serde_json::Value, WebError> {
    // Re-validate against the connected target's flash regions.
    let target = executor
        .call_async(OperationKind::TargetInfo, serde_json::json!({}))
        .await?;
    let regions = target
        .get("target")
        .and_then(|t| t.get("memory_regions"))
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|r| r.get("kind").and_then(|k| k.as_str()) == Some("nvm"))
                .filter_map(|r| {
                    let s = r.get("start").and_then(|v| v.as_u64())?;
                    let e = r.get("end").and_then(|v| v.as_u64())?;
                    Some((s, e))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let image = FirmwareImage::analyze(path, format, address)?;
    image.validate(&regions)?;

    executor
        .call_async(
            OperationKind::Flash,
            serde_json::json!({
                "action": "program",
                "path": path.to_string_lossy(),
                "format": format.as_str(),
                "address": image.segments[0].start,
                "verify": verify,
                "mode": mode,
            }),
        )
        .await
}
