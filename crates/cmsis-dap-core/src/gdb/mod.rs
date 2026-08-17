//! GDB Remote Serial Protocol server.
//!
//! Ported from [`probe-rs-tools`](https://github.com/probe-rs/probe-rs)
//! (MIT OR Apache-2.0, Copyright (c) probe-rs contributors) and adapted to
//! connect through this crate's options and error type. The connection is
//! non-invasive: it never resets the target unless `reset_halt` is set.

mod arch;
mod stub;
mod target;

pub(crate) use stub::{run, GdbInstanceConfiguration};

use crate::backend::Protocol;
use crate::error::{ErrorCode, McpError};
use parking_lot::FairMutex;
use probe_rs::probe::list::Lister;
use probe_rs::{Permissions, Session};
use std::path::PathBuf;
use std::time::Duration;

/// Options for starting a GDB server.
#[derive(Debug, Clone, Default)]
pub struct GdbServerOptions {
    pub probe_id: Option<String>,
    pub protocol: Option<Protocol>,
    pub speed_khz: Option<u32>,
    pub target: Option<String>,
    pub target_yaml: Option<PathBuf>,
    /// Reset and halt the core after attach (default: just attach, no reset).
    pub reset_halt: bool,
}

/// Attach (non-invasively) and serve GDB on `connection_string`
/// (default `127.0.0.1:1337`). Blocks until the server is stopped.
pub fn connect_and_serve(
    options: GdbServerOptions,
    connection_string: Option<&str>,
) -> Result<(), McpError> {
    let session = attach_session(&options)?;
    let instances = GdbInstanceConfiguration::from_session(&session, connection_string);
    for instance in &instances {
        eprintln!(
            "GDB stub for {:?} cores at {:?}",
            instance.core_type, instance.socket_addrs
        );
    }
    if instances.is_empty() {
        return Err(McpError::new(
            ErrorCode::UnsupportedFeature,
            "target has no cores to expose over GDB",
        ));
    }
    let session = FairMutex::new(session);
    run(&session, instances.iter())
        .map_err(|e| McpError::new(ErrorCode::ProtocolError, format!("GDB server error: {e}")))
}

fn attach_session(options: &GdbServerOptions) -> Result<Session, McpError> {
    let lister = Lister::new();
    let probes = lister.list_all();
    let selected = match &options.probe_id {
        Some(id) => probes
            .iter()
            .find(|p| p.serial_number.as_deref() == Some(id.as_str()) || p.identifier == *id)
            .ok_or_else(|| {
                McpError::new(ErrorCode::ProbeNotFound, format!("no probe with id {id}"))
            })?,
        None => probes
            .first()
            .ok_or_else(|| McpError::new(ErrorCode::ProbeNotFound, "no probe found"))?,
    };
    let mut probe = lister
        .open(selected)
        .map_err(|e| McpError::new(ErrorCode::ProbeNotFound, e.to_string()))?;
    probe
        .set_speed(options.speed_khz.unwrap_or(1000))
        .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
    let wire = match options.protocol.unwrap_or(Protocol::Swd) {
        Protocol::Swd => probe_rs::probe::WireProtocol::Swd,
        Protocol::Jtag => probe_rs::probe::WireProtocol::Jtag,
    };
    probe
        .select_protocol(wire)
        .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
    let registry = match &options.target_yaml {
        Some(path) => crate::backend::probe_rs::registry_from_yaml(path)?,
        None => probe_rs::config::Registry::from_builtin_families(),
    };
    let target_name = options
        .target
        .clone()
        .unwrap_or_else(|| "Cortex-M0".to_string());
    let mut session = probe
        .attach_with_registry(target_name, Permissions::default(), &registry)
        .map_err(|e| McpError::new(ErrorCode::ConnectFailed, e.to_string()))?;
    if options.reset_halt {
        session
            .core(0)
            .map_err(|e| McpError::new(ErrorCode::ConnectFailed, e.to_string()))?
            .reset_and_halt(Duration::from_millis(100))
            .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?;
    }
    Ok(session)
}
