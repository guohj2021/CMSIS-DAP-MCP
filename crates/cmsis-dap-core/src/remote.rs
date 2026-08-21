//! Remote TCP server: line-delimited JSON-RPC over TCP.
//!
//! The protocol mirrors the MCP tool names so a remote client (script, CLI or
//! another machine) can drive the same operations. One line per request:
//!
//! ```text
//! {"id":1,"method":"read_memory","params":{"address":0x20000000,"width":"u32","count":4}}
//! ```
//!
//! Responses are `{"id":1,"result":{...}}` or `{"id":1,"error":{"code":..,"message":..}}`.
//! Connection semantics are non-invasive: `connect` never resets the target.

use crate::backend::{AccessWidth, ConnectOptions, CoreRegister, Protocol, ResetMode};
use crate::error::{ErrorCode, McpError};
use crate::session::SessionManager;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

/// Serve remote JSON-RPC requests until the listener stops accepting.
pub async fn serve(session: &Arc<Mutex<SessionManager>>, bind: &str) -> Result<u16, McpError> {
    let listener = TcpListener::bind(bind)
        .await
        .map_err(|e| McpError::new(ErrorCode::ProtocolError, format!("TCP bind {bind}: {e}")))?;
    let port = listener
        .local_addr()
        .map_err(|e| McpError::new(ErrorCode::ProtocolError, e.to_string()))?
        .port();
    tracing::info!("remote TCP server listening on {bind}");
    serve_listener(listener, session).await?;
    Ok(port)
}

/// Serve remote JSON-RPC requests on an already-bound listener.
pub async fn serve_listener(
    listener: TcpListener,
    session: &Arc<Mutex<SessionManager>>,
) -> Result<(), McpError> {
    loop {
        let (stream, _peer) = match listener.accept().await {
            Ok(ok) => ok,
            Err(e) => {
                tracing::warn!("TCP accept failed: {e}");
                continue;
            }
        };
        let session = Arc::clone(session);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, session).await {
                tracing::warn!("remote connection error: {e}");
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    session: Arc<Mutex<SessionManager>>,
) -> Result<(), McpError> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(line) {
            Ok(request) => dispatch(&session, &request).await,
            Err(e) => json!({
                "id": Value::Null,
                "error": { "code": -32700, "message": format!("parse error: {e}") }
            }),
        };
        let mut payload = serde_json::to_string(&response)
            .map_err(|e| McpError::new(ErrorCode::InternalError, e.to_string()))?;
        payload.push('\n');
        if writer.write_all(payload.as_bytes()).await.is_err() {
            break;
        }
    }
    Ok(())
}

fn request_id(request: &Value) -> Value {
    request.get("id").cloned().unwrap_or(Value::Null)
}

fn rpc_error(id: Value, e: &McpError) -> Value {
    json!({
        "id": id,
        "error": { "code": format!("{:?}", e.code), "message": e.message }
    })
}

fn rpc_result(id: Value, result: Value) -> Value {
    json!({ "id": id, "result": result })
}

async fn dispatch(session: &Arc<Mutex<SessionManager>>, request: &Value) -> Value {
    let id = request_id(request);
    let method = match request.get("method").and_then(|m| m.as_str()) {
        Some(m) => m,
        None => {
            return json!({
                "id": id,
                "error": { "code": -32600, "message": "missing method" }
            })
        }
    };
    let params = request.get("params").cloned().unwrap_or(Value::Null);
    let mut session = match session.lock() {
        Ok(guard) => guard,
        Err(_) => {
            return rpc_error(
                id,
                &McpError::new(ErrorCode::InternalError, "session lock poisoned"),
            )
        }
    };
    let result = match method {
        "list_probes" => session
            .backend()
            .list_probes()
            .map(|probes| json!({ "probes": probes })),
        "connect" => connect(&mut session, &params).map(|info| json!({ "target": info })),
        "disconnect" => session
            .disconnect()
            .map(|_| json!({ "disconnected": true })),
        "read_memory" => {
            read_memory(&mut session, &params).map(|values| json!({ "values": values }))
        }
        "write_memory" => write_memory(&mut session, &params).map(|_| json!({ "written": true })),
        "read_core_register" => read_register(&mut session, &params)
            .map(|(name, value)| json!({ "register": name, "value": value })),
        "halt" => session.backend().halt().map(|_| json!({ "halted": true })),
        "resume" => session
            .backend()
            .resume()
            .map(|_| json!({ "running": true })),
        "step" => session.backend().step().map(|_| json!({ "stepped": true })),
        "reset" => reset(&mut session, &params).map(|mode| json!({ "mode": mode })),
        "status" => session
            .backend()
            .get_core_status()
            .map(|s| json!({ "status": s })),
        "dump_cpu_state" => dump(&mut session, &params),
        other => Err(McpError::new(
            ErrorCode::InvalidArgument,
            format!("unknown method '{other}'"),
        )),
    };
    match result {
        Ok(value) => rpc_result(id, value),
        Err(e) => rpc_error(id, &e),
    }
}

fn connect(
    session: &mut SessionManager,
    params: &Value,
) -> Result<crate::backend::TargetInfo, McpError> {
    let opts = ConnectOptions {
        probe_id: params
            .get("probe_id")
            .and_then(|v| v.as_str())
            .map(String::from),
        protocol: parse_protocol(
            params
                .get("protocol")
                .and_then(|v| v.as_str())
                .unwrap_or("swd"),
        )?,
        speed_khz: params
            .get("speed_khz")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        target: params
            .get("target")
            .and_then(|v| v.as_str())
            .map(String::from),
        under_reset: params
            .get("under_reset")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        core_index: params
            .get("core")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize),
    };
    session.connect(&opts)
}

fn parse_protocol(s: &str) -> Result<Protocol, McpError> {
    match s {
        "swd" => Ok(Protocol::Swd),
        "jtag" => Ok(Protocol::Jtag),
        other => Err(McpError::new(
            ErrorCode::InvalidArgument,
            format!("protocol must be swd or jtag, got {other}"),
        )),
    }
}

fn parse_width(s: &str) -> Result<AccessWidth, McpError> {
    match s {
        "u8" => Ok(AccessWidth::U8),
        "u16" => Ok(AccessWidth::U16),
        "u32" => Ok(AccessWidth::U32),
        "u64" => Ok(AccessWidth::U64),
        other => Err(McpError::new(
            ErrorCode::InvalidArgument,
            format!("width must be u8/u16/u32/u64, got {other}"),
        )),
    }
}

fn read_memory(session: &mut SessionManager, params: &Value) -> Result<Vec<u64>, McpError> {
    session.ensure_connected()?;
    let address = params
        .get("address")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| McpError::new(ErrorCode::InvalidArgument, "missing address"))?;
    let width = parse_width(
        params
            .get("width")
            .and_then(|v| v.as_str())
            .unwrap_or("u32"),
    )?;
    let count = params
        .get("count")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .min(1 << 20) as u32;
    session.backend().read_memory(address, width, count)
}

fn write_memory(session: &mut SessionManager, params: &Value) -> Result<(), McpError> {
    session.ensure_connected()?;
    let address = params
        .get("address")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| McpError::new(ErrorCode::InvalidArgument, "missing address"))?;
    let width = parse_width(
        params
            .get("width")
            .and_then(|v| v.as_str())
            .unwrap_or("u32"),
    )?;
    let values: Vec<u64> = params
        .get("values")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_u64()).collect())
        .ok_or_else(|| McpError::new(ErrorCode::InvalidArgument, "missing values"))?;
    session.backend().write_memory(address, width, &values)
}

fn read_register(session: &mut SessionManager, params: &Value) -> Result<(String, u64), McpError> {
    session.ensure_connected()?;
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::new(ErrorCode::InvalidArgument, "missing register name"))?
        .to_string();
    let value = session
        .backend()
        .read_core_register(&CoreRegister::Name(name.clone()))?;
    Ok((name, value))
}

fn reset(session: &mut SessionManager, params: &Value) -> Result<String, McpError> {
    let mode = match params.get("mode").and_then(|v| v.as_str()).unwrap_or("run") {
        "run" => ResetMode::Run,
        "halt" => ResetMode::Halt,
        other => {
            return Err(McpError::new(
                ErrorCode::InvalidArgument,
                format!("reset mode must be run or halt, got {other}"),
            ))
        }
    };
    session.backend().reset(mode)?;
    Ok(if mode == ResetMode::Run {
        "run"
    } else {
        "halt"
    }
    .into())
}

fn dump(session: &mut SessionManager, params: &Value) -> Result<Value, McpError> {
    session.ensure_connected()?;
    let addresses: Vec<u64> = params
        .get("addresses")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_u64()).collect())
        .unwrap_or_default();
    let stack_words = params
        .get("stack_words")
        .and_then(|v| v.as_u64())
        .unwrap_or(16) as usize;
    let restore = params
        .get("restore")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let dump = session
        .backend()
        .dump_cpu_state(&addresses, stack_words, restore)?;
    serde_json::to_value(dump).map_err(|e| McpError::new(ErrorCode::InternalError, e.to_string()))
}
