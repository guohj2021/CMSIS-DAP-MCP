//! Keil FLM -> probe-rs target YAML generation.
//!
//! An FLM is an ARM ELF containing the vendor flash programming algorithm
//! (code segment) plus a `FlashDevice` descriptor (usually in a separate data
//! segment). This module extracts the algorithm bytes, the entry-point
//! symbols and the descriptor fields, so a usable target YAML can be produced
//! from just the FLM plus the Flash/SRAM address ranges.

// FLM parsing and YAML generation moved to cmsis_dap_core::flm.
// Re-export for backward compatibility with existing CLI callers.
pub use cmsis_dap_core::flm::{generate_yaml, parse_flm, FlashDevice, FlmParse};
