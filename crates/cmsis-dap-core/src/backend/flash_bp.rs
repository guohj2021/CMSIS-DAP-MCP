//! Flash software breakpoints.
//!
//! Hardware breakpoints are limited (typically 4-6 DWT comparators on
//! Cortex-M). Flash software breakpoints trade that limit for flash
//! modification: the original instruction at a flash address is replaced
//! with a Thumb `BKPT` (`0xBE00`), and restored when the breakpoint is
//! cleared. Because they modify flash contents, they are destructive and
//! must be gated behind the destructive policy.
//!
//! ARM-mode (32-bit) breakpoints are not supported yet; only the Thumb-2
//! 16-bit `BKPT` encoding is used.

use std::collections::BTreeMap;

/// Thumb `BKPT` instruction, little-endian bytes (`0xBE00`).
pub const THUMB_BKPT: [u8; 2] = [0x00, 0xBE];

/// Tracks active flash breakpoints: address -> original instruction bytes.
#[derive(Debug, Default)]
pub struct FlashBpManager {
    active: BTreeMap<u64, Vec<u8>>,
}

impl FlashBpManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self, address: u64) -> bool {
        self.active.contains_key(&address)
    }

    pub fn addresses(&self) -> Vec<u64> {
        self.active.keys().copied().collect()
    }

    /// Return the original instruction bytes remembered for `address`, if
    /// the breakpoint is active.
    pub fn get(&self, address: u64) -> Option<Vec<u8>> {
        self.active.get(&address).cloned()
    }

    /// Register a breakpoint at `address`, remembering the `original`
    /// instruction bytes it replaces.
    pub fn insert(&mut self, address: u64, original: Vec<u8>) {
        self.active.entry(address).or_insert(original);
    }

    /// Remove the breakpoint at `address`, returning the original bytes to
    /// restore, if it was active.
    pub fn remove(&mut self, address: u64) -> Option<Vec<u8>> {
        self.active.remove(&address)
    }
}
