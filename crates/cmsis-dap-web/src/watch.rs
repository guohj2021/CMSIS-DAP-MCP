//! Watch items + WatchTarget model (frozen v5 §5).

use serde::{Deserialize, Serialize};

/// User-facing watch target; resolved at read time via the DebugResolver.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WatchTarget {
    Symbol { symbol_id: u64 },
    Address { address: u64 },
    Register { name: String },
}

/// A watch item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchItem {
    pub id: u64,
    pub target: WatchTarget,
    /// auto | hex | dec | bin
    #[serde(default = "default_format")]
    pub format: String,
    /// Refresh interval in ms (live watch / periodic refresh).
    #[serde(default = "default_rate")]
    pub rate_ms: u32,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_format() -> String {
    "auto".into()
}
fn default_rate() -> u32 {
    500
}
fn default_true() -> bool {
    true
}
