//! Peripheral monitor items (frozen v5 §8 MonitorSafety).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonitorSafety {
    Safe,
    UserConfirmed,
    NotRecommended,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorItem {
    pub id: u64,
    pub peripheral: String,
    pub register: String,
    pub rate_ms: u32,
    pub safety: MonitorSafety,
}
