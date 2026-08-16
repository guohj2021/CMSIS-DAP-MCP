//! Host-side decoder for the CMSIS-View Event Recorder.
//!
//! The layouts here are locked to the official `EventRecorder.c` (protocol
//! version 1.x) so the debugger can read the on-chip circular buffer over
//! plain SWD/JTAG memory accesses, with no trace hardware required:
//!
//! - `EventRecorderInfo`: 24 bytes (protocol, record count, pointers).
//! - `EventStatus`: 36 bytes (record index, timestamps, statistics).
//! - `EventRecord`: 16 bytes per event (ts, val1, val2, info).
//!
//! A record is only committed after its `info` word is written (VALID set,
//! LOCKED cleared), so the host can safely poll while the target is running.

use crate::backend::EvrEvent;
use crate::error::{ErrorCode, McpError};

pub const INFO_SIZE: usize = 24;
pub const STATUS_SIZE: usize = 36;
pub const RECORD_SIZE: usize = 16;

// EventRecorderInfo offsets.
pub const INFO_PROTOCOL_TYPE: usize = 0;
pub const INFO_PROTOCOL_VERSION: usize = 2;
pub const INFO_RECORD_COUNT: usize = 4;
pub const INFO_EVENT_BUFFER: usize = 8;
pub const INFO_EVENT_FILTER: usize = 12;
pub const INFO_EVENT_STATUS: usize = 16;
pub const INFO_TS_SOURCE: usize = 20;

// EventStatus offsets.
pub const STATUS_STATE: usize = 0;
pub const STATUS_CONTEXT: usize = 1;
pub const STATUS_INFO_CRC: usize = 2;
pub const STATUS_RECORD_INDEX: usize = 4;
pub const STATUS_RECORDS_WRITTEN: usize = 8;
pub const STATUS_RECORDS_DUMPED: usize = 12;
pub const STATUS_TS_OVERFLOW: usize = 16;
pub const STATUS_TS_FREQ: usize = 20;
pub const STATUS_TS_LAST: usize = 24;
pub const STATUS_INIT_COUNT: usize = 28;
pub const STATUS_SIGNATURE: usize = 32;

// EventRecord offsets.
pub const RECORD_TS: usize = 0;
pub const RECORD_VAL1: usize = 4;
pub const RECORD_VAL2: usize = 8;
pub const RECORD_INFO: usize = 12;

// Record `info` bit masks.
pub const INFO_FIRST: u32 = 0x0100_0000;
pub const INFO_LAST: u32 = 0x0200_0000;
pub const INFO_LOCKED: u32 = 0x0400_0000;
pub const INFO_VALID: u32 = 0x0800_0000;
pub const INFO_MSB_TS: u32 = 0x1000_0000;
pub const INFO_MSB_VAL1: u32 = 0x2000_0000;
pub const INFO_MSB_VAL2: u32 = 0x4000_0000;

/// Parsed `EventRecorderInfo` header.
#[derive(Debug, Clone)]
pub struct EvrInfoHeader {
    pub protocol_version: String,
    pub record_count: u32,
    pub event_buffer: u64,
    pub event_status: u64,
    pub ts_source: u8,
}

/// Parsed `EventStatus` fields.
#[derive(Debug, Clone)]
pub struct EvrStatusFields {
    pub state: u8,
    pub record_index: u32,
    pub records_written: u32,
    pub records_dumped: u32,
    pub ts_overflow: u32,
    pub ts_freq: u32,
    pub init_count: u32,
    pub signature: u32,
}

fn u16_le(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([b[off], b[off + 1]])
}

fn u32_le(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

fn evr_error(msg: impl Into<String>) -> McpError {
    McpError::new(ErrorCode::UnsupportedFeature, msg)
}

/// Parse the 24-byte `EventRecorderInfo` structure and validate the protocol.
pub fn parse_header(bytes: &[u8]) -> Result<EvrInfoHeader, McpError> {
    if bytes.len() < INFO_SIZE {
        return Err(evr_error("EventRecorderInfo is shorter than 24 bytes"));
    }
    if bytes[INFO_PROTOCOL_TYPE] != 1 {
        return Err(evr_error(format!(
            "unsupported Event Recorder protocol type {} (expected 1 = DAP)",
            bytes[INFO_PROTOCOL_TYPE]
        )));
    }
    let version = u16_le(bytes, INFO_PROTOCOL_VERSION);
    let major = version >> 8;
    if major != 1 {
        return Err(evr_error(format!(
            "unsupported Event Recorder protocol version {}.{} (expected 1.x)",
            major,
            version & 0xFF
        )));
    }
    Ok(EvrInfoHeader {
        protocol_version: format!("{}.{}", major, version & 0xFF),
        record_count: u32_le(bytes, INFO_RECORD_COUNT),
        event_buffer: u32_le(bytes, INFO_EVENT_BUFFER) as u64,
        event_status: u32_le(bytes, INFO_EVENT_STATUS) as u64,
        ts_source: bytes[INFO_TS_SOURCE],
    })
}

/// Parse the 36-byte `EventStatus` structure.
pub fn parse_status(bytes: &[u8]) -> Result<EvrStatusFields, McpError> {
    if bytes.len() < STATUS_SIZE {
        return Err(McpError::new(
            ErrorCode::MemoryFault,
            "EventStatus is shorter than 36 bytes",
        ));
    }
    Ok(EvrStatusFields {
        state: bytes[STATUS_STATE],
        record_index: u32_le(bytes, STATUS_RECORD_INDEX),
        records_written: u32_le(bytes, STATUS_RECORDS_WRITTEN),
        records_dumped: u32_le(bytes, STATUS_RECORDS_DUMPED),
        ts_overflow: u32_le(bytes, STATUS_TS_OVERFLOW),
        ts_freq: u32_le(bytes, STATUS_TS_FREQ),
        init_count: u32_le(bytes, STATUS_INIT_COUNT),
        signature: u32_le(bytes, STATUS_SIGNATURE),
    })
}

/// Decode one 16-byte event record.
///
/// Returns `None` when the record is still being written (LOCKED) or has not
/// been committed (VALID unset); callers must not advance their read index
/// past such a record.
pub fn decode_record(bytes: &[u8], ts_overflow: u32, ts_freq: u32) -> Option<EvrEvent> {
    if bytes.len() < RECORD_SIZE {
        return None;
    }
    let info = u32_le(bytes, RECORD_INFO);
    if info & INFO_LOCKED != 0 || info & INFO_VALID == 0 {
        return None;
    }
    let ts = (u32_le(bytes, RECORD_TS) & 0x7FFF_FFFF)
        | if info & INFO_MSB_TS != 0 {
            0x8000_0000
        } else {
            0
        };
    let val1 = (u32_le(bytes, RECORD_VAL1) & 0x7FFF_FFFF)
        | if info & INFO_MSB_VAL1 != 0 {
            0x8000_0000
        } else {
            0
        };
    let val2 = (u32_le(bytes, RECORD_VAL2) & 0x7FFF_FFFF)
        | if info & INFO_MSB_VAL2 != 0 {
            0x8000_0000
        } else {
            0
        };
    let timestamp_ticks = ((ts_overflow as u64) << 32) | (ts as u64);
    let timestamp_secs = if ts_freq > 0 {
        timestamp_ticks as f64 / ts_freq as f64
    } else {
        0.0
    };
    Some(EvrEvent {
        timestamp_ticks,
        timestamp_secs,
        context: ((info >> 16) & 0x7) as u8,
        component: ((info >> 8) & 0xFF) as u16,
        message: (info & 0xFF) as u16,
        irq: (info >> 19) & 1 == 1,
        first: info & INFO_FIRST != 0,
        last: info & INFO_LAST != 0,
        sequence: ((info >> 20) & 0xF) as u8,
        val1,
        val2,
    })
}

/// Tracks how many event records the host has consumed.
///
/// The target's `record_index` is an allocation counter (not a slot index);
/// the firmware increments it before writing each record. Global index `i`
/// maps to slot `i % record_count`.
#[derive(Debug, Clone)]
pub struct EvrReader {
    pub event_buffer: u64,
    pub event_status: u64,
    pub record_count: u32,
    pub ts_overflow: u32,
    pub ts_freq: u32,
    pub last_index: u32,
}

impl EvrReader {
    pub fn new(header: &EvrInfoHeader) -> Self {
        Self {
            event_buffer: header.event_buffer,
            event_status: header.event_status,
            record_count: header.record_count.max(1),
            ts_overflow: 0,
            ts_freq: 0,
            last_index: 0,
        }
    }

    /// Plan which global record indices are new, and update internal state.
    ///
    /// If more records were produced than fit in the ring buffer, only the
    /// newest `record_count` entries are returned (older ones were
    /// overwritten); the caller still advances past the dropped range.
    pub fn plan(&mut self, status: &EvrStatusFields) -> Vec<u32> {
        let delta = status.record_index.wrapping_sub(self.last_index);
        if delta == 0 {
            return Vec::new();
        }
        let count = self.record_count;
        let available = if delta > count { count } else { delta };
        let start = status.record_index.wrapping_sub(available);
        self.last_index = start;
        (0..available).map(|k| start.wrapping_add(k)).collect()
    }

    /// Mark record `index` as consumed.
    pub fn advance(&mut self, index: u32) {
        self.last_index = index.wrapping_add(1);
    }
}
