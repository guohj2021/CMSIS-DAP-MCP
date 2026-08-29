//! REST API (v5 §11): Query / Command over the executor.

use crate::executor::ExecutorHandle;
use crate::op::{OperationKind, WebError};
use crate::state::SharedState;
use axum::extract::{Multipart, Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/probes", get(probes))
        .route("/api/connect", post(connect))
        .route("/api/disconnect", post(disconnect))
        .route("/api/status", get(status))
        .route("/api/debug/run", post(debug_run))
        .route("/api/debug/halt", post(debug_halt))
        .route("/api/debug/step", post(debug_step))
        .route("/api/debug/reset", post(debug_reset))
        .route("/api/registers", get(registers).post(register_write))
        .route("/api/memory/read", post(memory_read))
        .route("/api/memory/write", post(memory_write))
        .route("/api/breakpoints", get(breakpoints).post(breakpoint_set))
        .route("/api/breakpoints/limits", get(breakpoint_limits))
        .route("/api/breakpoints/{address}", axum::routing::delete(breakpoint_delete))
        .route("/api/watchpoints", get(watchpoints).post(watchpoint_set))
        .route("/api/watchpoints/{address}", axum::routing::delete(watchpoint_delete))
        .route("/api/fault", get(fault))
        .route("/api/snapshot", post(snapshot))
        .route("/api/files/firmware", post(firmware_upload))
        .route("/api/flash/erase", post(flash_erase))
        .route("/api/flash/program", post(flash_program))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health(State(state): State<SharedState>) -> Response {
    api_result(Ok(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "state": state.executor.status(),
    })))
}

async fn probes(State(state): State<SharedState>) -> Response {
    let ex = state.executor.clone();
    api_result(ex.call_async(OperationKind::ListProbes, json!({})).await)
}

#[derive(Deserialize)]
struct ConnectBody {
    probe_id: Option<String>,
    protocol: Option<String>,
    speed_khz: Option<u32>,
    target: Option<String>,
    under_reset: Option<bool>,
    core_index: Option<usize>,
}

async fn connect(State(state): State<SharedState>, Json(body): Json<ConnectBody>) -> Response {
    let ex = state.executor.clone();
    let d = &state.default_connect;
    let params = json!({
        "probe_id": body.probe_id.or_else(|| d.probe_id.clone()),
        "protocol": body.protocol.or_else(|| Some(match d.protocol {
            cmsis_dap_core::backend::Protocol::Swd => "swd".into(),
            cmsis_dap_core::backend::Protocol::Jtag => "jtag".into(),
        })),
        "speed_khz": body.speed_khz.or(d.speed_khz),
        "target": body.target.or_else(|| d.target.clone()),
        "under_reset": body.under_reset.or(Some(d.under_reset)),
        "core_index": body.core_index.or(d.core_index),
    });
    api_result(ex.call_async(OperationKind::Connect, params).await)
}

async fn disconnect(State(state): State<SharedState>) -> Response {
    let ex = state.executor.clone();
    api_result(ex.call_async(OperationKind::Disconnect, json!({})).await)
}

async fn status(State(state): State<SharedState>) -> Response {
    let ex = state.executor.clone();
    api_result(ex.call_async(OperationKind::Status, json!({})).await.map(|v| {
        // Enrich with the three-layer model.
        let s = ex.status();
        json!({ "status": v.get("status"), "server": s.server, "target": s.target, "operation": s.operation, "reason": s.reason, "pc": s.pc })
    }))
}

async fn debug_run(State(state): State<SharedState>) -> Response {
    api_result(state.executor.clone().call_async(OperationKind::Run, json!({})).await)
}
async fn debug_halt(State(state): State<SharedState>) -> Response {
    api_result(state.executor.clone().call_async(OperationKind::Halt, json!({})).await)
}
async fn debug_step(State(state): State<SharedState>) -> Response {
    api_result(state.executor.clone().call_async(OperationKind::Step, json!({})).await)
}

#[derive(Deserialize)]
struct ResetBody {
    mode: Option<String>,
}
async fn debug_reset(State(state): State<SharedState>, Json(body): Json<ResetBody>) -> Response {
    api_result(
        state
            .executor
            .clone()
            .call_async(OperationKind::Reset, json!({ "mode": body.mode }))
            .await,
    )
}

async fn registers(State(state): State<SharedState>) -> Response {
    api_result(state.executor.clone().call_async(OperationKind::ListRegisters, json!({})).await)
}

#[derive(Deserialize)]
struct RegisterWriteBody {
    name: String,
    value: u64,
}
async fn register_write(State(state): State<SharedState>, Json(body): Json<RegisterWriteBody>) -> Response {
    api_result(
        state
            .executor
            .clone()
            .call_async(OperationKind::RegisterWrite, json!({ "name": body.name, "value": body.value }))
            .await,
    )
}

#[derive(Deserialize)]
struct MemoryReadBody {
    address: u64,
    width: Option<String>,
    count: Option<u32>,
}
async fn memory_read(State(state): State<SharedState>, Json(body): Json<MemoryReadBody>) -> Response {
    api_result(
        state
            .executor
            .clone()
            .call_async(
                OperationKind::MemoryRead,
                json!({ "address": body.address, "width": body.width, "count": body.count }),
            )
            .await,
    )
}

#[derive(Deserialize)]
struct MemoryWriteBody {
    address: u64,
    width: Option<String>,
    values: Vec<u64>,
}
async fn memory_write(State(state): State<SharedState>, Json(body): Json<MemoryWriteBody>) -> Response {
    api_result(
        state
            .executor
            .clone()
            .call_async(
                OperationKind::MemoryWrite,
                json!({ "address": body.address, "width": body.width, "values": body.values }),
            )
            .await,
    )
}

async fn breakpoints(State(state): State<SharedState>) -> Response {
    api_result(
        state
            .executor
            .clone()
            .call_async(OperationKind::Breakpoint, json!({ "action": "list" }))
            .await,
    )
}

#[derive(Deserialize)]
struct BreakpointBody {
    address: u64,
    kind: Option<String>,
}
async fn breakpoint_set(State(state): State<SharedState>, Json(body): Json<BreakpointBody>) -> Response {
    let action = match body.kind.as_deref() {
        Some("sw_flash") | Some("flash") => "set_flash",
        _ => "set",
    };
    api_result(
        state
            .executor
            .clone()
            .call_async(OperationKind::Breakpoint, json!({ "action": action, "address": body.address }))
            .await,
    )
}

async fn breakpoint_delete(State(state): State<SharedState>, Path(_address): Path<u64>) -> Response {
    api_result(
        state
            .executor
            .clone()
            .call_async(OperationKind::Breakpoint, json!({ "action": "clear" }))
            .await,
    )
}

async fn breakpoint_limits(State(state): State<SharedState>) -> Response {
    api_result(
        state
            .executor
            .clone()
            .call_async(OperationKind::BreakpointLimits, json!({}))
            .await,
    )
}

async fn watchpoints(State(state): State<SharedState>) -> Response {
    api_result(
        state
            .executor
            .clone()
            .call_async(OperationKind::Watchpoint, json!({ "action": "list" }))
            .await,
    )
}

#[derive(Deserialize)]
struct WatchpointBody {
    address: u64,
    access: Option<String>,
}
async fn watchpoint_set(State(state): State<SharedState>, Json(body): Json<WatchpointBody>) -> Response {
    api_result(
        state
            .executor
            .clone()
            .call_async(
                OperationKind::Watchpoint,
                json!({ "action": "set", "address": body.address, "access": body.access }),
            )
            .await,
    )
}

async fn watchpoint_delete(State(state): State<SharedState>, Path(_address): Path<u64>) -> Response {
    api_result(
        state
            .executor
            .clone()
            .call_async(OperationKind::Watchpoint, json!({ "action": "clear" }))
            .await,
    )
}

async fn fault(State(state): State<SharedState>) -> Response {
    api_result(state.executor.clone().call_async(OperationKind::DumpFault, json!({})).await)
}

async fn snapshot(State(state): State<SharedState>) -> Response {
    api_result(state.executor.clone().call_async(OperationKind::Snapshot, json!({})).await)
}

// ---------------------------------------------------------------------------
// Firmware upload + flash (v5 §7)
// ---------------------------------------------------------------------------

async fn firmware_upload(State(state): State<SharedState>, mut multipart: Multipart) -> Response {
    let mut data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut bin_address: Option<u64> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            file_name = field.file_name().map(|s| s.to_string());
            data = Some(match field.bytes().await {
                Ok(b) => b.to_vec(),
                Err(e) => {
                    return api_result(Err(WebError::Internal(format!("upload read failed: {e}"))))
                }
            });
        } else if name == "address" {
            if let Ok(text) = field.text().await {
                bin_address = text.trim().parse::<u64>().ok();
            }
        }
    }
    let Some(data) = data else {
        return api_result(Err(WebError::InvalidArgument("missing file field".into())));
    };
    let Some(file_name) = file_name else {
        return api_result(Err(WebError::InvalidArgument("missing file name".into())));
    };

    let format = match crate::flash::FirmwareFormat::from_extension(std::path::Path::new(&file_name)) {
        Ok(f) => f,
        Err(e) => return api_result(Err(e)),
    };
    let id = uuid::Uuid::new_v4().to_string();
    let ext = format.as_str();
    let path = state._upload_dir.path().join(format!("{id}.{ext}"));
    if let Err(e) = std::fs::write(&path, &data) {
        return api_result(Err(WebError::Internal(format!("failed to save upload: {e}"))));
    }
    state.uploads.lock().unwrap().insert(id.clone(), path.clone());

    match crate::flash::FirmwareImage::analyze(&path, format, bin_address) {
        Ok(image) => api_result(Ok(serde_json::json!({
            "file_id": id,
            "format": format.as_str(),
            "total_size": image.total_size,
            "address_range": image.address_range,
            "segments": image.segments.iter().map(|s| serde_json::json!({
                "start": s.start, "end": s.end, "size": s.size()
            })).collect::<Vec<_>>(),
        }))),
        Err(e) => api_result(Err(e)),
    }
}

#[derive(Deserialize)]
struct FlashEraseBody {
    address: u64,
    size: u64,
}
async fn flash_erase(State(state): State<SharedState>, Json(body): Json<FlashEraseBody>) -> Response {
    api_result(
        state
            .executor
            .clone()
            .call_async(
                OperationKind::Flash,
                json!({ "action": "erase", "address": body.address, "size": body.size }),
            )
            .await,
    )
}

#[derive(Deserialize)]
struct FlashProgramBody {
    file_id: String,
    address: Option<u64>,
    verify: Option<bool>,
    mode: Option<String>,
}
async fn flash_program(State(state): State<SharedState>, Json(body): Json<FlashProgramBody>) -> Response {
    let path = state
        .uploads
        .lock()
        .unwrap()
        .get(&body.file_id)
        .cloned();
    let Some(path) = path else {
        return api_result(Err(WebError::InvalidArgument(format!(
            "unknown file_id {}",
            body.file_id
        ))));
    };
    let format = match crate::flash::FirmwareFormat::from_extension(&path) {
        Ok(f) => f,
        Err(e) => return api_result(Err(e)),
    };
    let result = crate::flash::program_image(
        &state.executor,
        &body.file_id,
        body.address,
        body.verify.unwrap_or(true),
        body.mode.as_deref().unwrap_or("stay_halted"),
        format,
        &path,
    )
    .await;
    api_result(result)
}
// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

fn api_result(result: Result<Value, WebError>) -> Response {
    match result {
        Ok(v) => Json(v).into_response(),
        Err(e) => {
            let status = e.http_status();
            (status, Json(e.to_json())).into_response()
        }
    }
}

// Keep ExecutorHandle import used (handlers rely on it via state).
#[allow(dead_code)]
fn _executor_type(_: &ExecutorHandle) {}



