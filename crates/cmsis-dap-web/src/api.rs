//! REST API (v5 §11): Query / Command over the executor.

use crate::executor::ExecutorHandle;
use crate::op::{OperationKind, ServerEvent, WebError};
use crate::state::SharedState;
use axum::extract::{Multipart, Path, Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use capstone::prelude::*;
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
        .route("/api/files/elf", post(elf_upload))
        .route("/api/symbols", get(symbols))
        .route("/api/symbols/resolve", get(symbols_resolve))
        .route("/api/watch", get(watch_list).post(watch_add))
        .route("/api/watch/{id}", axum::routing::delete(watch_delete).patch(watch_patch))
        .route("/api/watch/refresh", post(watch_refresh))
        .route("/api/peripherals/monitor", get(monitor_list).post(monitor_add))
        .route("/api/peripherals/monitor/{id}", axum::routing::delete(monitor_delete))
        .route("/api/rtt/start", post(rtt_start))
        .route("/api/rtt/stop", post(rtt_stop))
        .route("/api/rtt/channels", get(rtt_channels))
        .route("/api/evr/start", post(evr_start))
        .route("/api/evr/stop", post(evr_stop))
        .route("/api/disassembly", get(disassembly))
        .route("/api/address/{address}", get(address_describe))
        .route("/api/svd", post(svd_upload))
        .route("/api/peripherals", get(peripherals))
        .route("/api/peripherals/{name}", get(peripheral_get))
        .route("/api/peripherals/{name}/read", post(peripheral_read))
        .route("/api/peripherals/{name}/write", post(peripheral_write))
        .route("/api/peripherals/{name}/decode", post(peripheral_decode))
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
    address: Option<u64>,
    kind: Option<String>,
    /// { "kind": "symbol", "name": "main" } resolves via the loaded ELF.
    target: Option<Value>,
}
async fn breakpoint_set(State(state): State<SharedState>, Json(body): Json<BreakpointBody>) -> Response {
    let action = match body.kind.as_deref() {
        Some("sw_flash") | Some("flash") => "set_flash",
        _ => "set",
    };
    let address = if let Some(t) = &body.target {
        if t.get("kind").and_then(|k| k.as_str()) == Some("symbol") {
            let name = t.get("name").and_then(|n| n.as_str());
            let Some(name) = name else {
                return api_result(Err(WebError::InvalidArgument("symbol target needs name".into())));
            };
            let sym = state
                .symbols
                .read()
                .unwrap()
                .as_ref()
                .and_then(|db| db.resolve_name(name))
                .map(|s| s.address);
            match sym {
                Some(a) => a,
                None => {
                    return api_result(Err(WebError::InvalidArgument(format!(
                        "symbol {name} not found in loaded ELF"
                    ))))
                }
            }
        } else {
            return api_result(Err(WebError::InvalidArgument(
                "target must be {kind:symbol, name} or use address".into(),
            )));
        }
    } else {
        let Some(a) = body.address else {
            return api_result(Err(WebError::InvalidArgument(
                "breakpoint requires address or target".into(),
            )));
        };
        a
    };
    api_result(
        state
            .executor
            .clone()
            .call_async(OperationKind::Breakpoint, json!({ "action": action, "address": address }))
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
// ELF / Symbols / Watch / SVD (frozen v5 §5, §8, §9)
// ---------------------------------------------------------------------------

async fn elf_upload(State(state): State<SharedState>, mut multipart: Multipart) -> Response {
    let mut data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name().unwrap_or("") == "file" {
            file_name = field.file_name().map(|s| s.to_string());
            data = Some(match field.bytes().await {
                Ok(b) => b.to_vec(),
                Err(e) => {
                    return api_result(Err(WebError::Internal(format!("upload read failed: {e}"))))
                }
            });
        }
    }
    let Some(data) = data else {
        return api_result(Err(WebError::InvalidArgument("missing file field".into())));
    };
    let Some(file_name) = file_name else {
        return api_result(Err(WebError::InvalidArgument("missing file name".into())));
    };
    let id = uuid::Uuid::new_v4().to_string();
    let path = state._upload_dir.path().join(&id);
    if let Err(e) = std::fs::write(&path, &data) {
        return api_result(Err(WebError::Internal(format!("failed to save upload: {e}"))));
    }
    let _ = file_name;
    let db = match cmsis_dap_core::symbols::SymbolDatabase::load(&path) {
        Ok(db) => db,
        Err(e) => return api_result(Err(WebError::InvalidArgument(e.to_string()))),
    };
    let (functions, variables) = {
        let (f, v) = db
            .symbols()
            .iter()
            .fold((0usize, 0usize), |(f, v), s| match s.kind {
                cmsis_dap_core::symbols::SymbolKind::Function => (f + 1, v),
                cmsis_dap_core::symbols::SymbolKind::Variable => (f, v + 1),
                _ => (f, v),
            });
        (f, v)
    };
    *state.symbols.write().unwrap() = Some(db.clone());
    // Load DWARF source locations (best effort; None when absent).
    let debug_info = cmsis_dap_core::symbols::DebugInfo::load(&path).ok().flatten();
    *state.debug_info.lock().unwrap() = debug_info;
    api_result(Ok(json!({
        "file_id": id,
        "symbols": state.symbols.read().unwrap().as_ref().map(|d| d.len()).unwrap_or(0),
        "functions": functions,
        "variables": variables,
    })))
}

#[derive(Deserialize)]
struct SymbolsQuery {
    kind: Option<String>,
    pattern: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
}
async fn symbols(State(state): State<SharedState>, Query(q): Query<SymbolsQuery>) -> Response {
    let Some(db) = state.symbols.read().unwrap().clone() else {
        return api_result(Ok(json!({ "total": 0, "items": [] })));
    };
    let kind = match q.kind.as_deref() {
        None | Some("") => None,
        Some("function") => Some(cmsis_dap_core::symbols::SymbolKind::Function),
        Some("variable") => Some(cmsis_dap_core::symbols::SymbolKind::Variable),
        Some(other) => {
            return api_result(Err(WebError::InvalidArgument(format!(
                "kind must be function|variable, got {other}"
            ))))
        }
    };
    let (total, page) = db.search(kind, q.pattern.as_deref(), q.offset.unwrap_or(0), q.limit.unwrap_or(200));
    api_result(Ok(json!({
        "total": total,
        "items": page.iter().map(|s| json!({
            "id": s.id, "name": s.name, "address": s.address, "size": s.size,
            "kind": s.kind, "section": s.section, "module": s.module,
        })).collect::<Vec<_>>(),
    })))
}

#[derive(Deserialize)]
struct SymbolsResolveQuery {
    name: String,
}
async fn symbols_resolve(State(state): State<SharedState>, Query(q): Query<SymbolsResolveQuery>) -> Response {
    let Some(db) = state.symbols.read().unwrap().clone() else {
        return api_result(Err(WebError::InvalidArgument("no ELF loaded".into())));
    };
    match db.resolve_name(&q.name) {
        Some(s) => api_result(Ok(json!({
            "symbol": { "id": s.id, "name": s.name, "address": s.address, "size": s.size, "kind": s.kind }
        }))),
        None => api_result(Err(WebError::InvalidArgument(format!("symbol {} not found", q.name)))),
    }
}

// --- Watch ---

#[derive(Deserialize)]
struct WatchAddBody {
    target: Value,
    format: Option<String>,
    rate_ms: Option<u32>,
}
async fn watch_add(State(state): State<SharedState>, Json(body): Json<WatchAddBody>) -> Response {
    let target = match body.target.get("kind").and_then(|k| k.as_str()) {
        Some("symbol") => {
            let id = body.target.get("symbol_id").and_then(|v| v.as_u64());
            let name = body.target.get("name").and_then(|v| v.as_str()).map(String::from);
            let resolved_id = match id {
                Some(i) => i,
                None => {
                    let Some(name) = name else {
                        return api_result(Err(WebError::InvalidArgument(
                            "symbol target needs symbol_id or name".into(),
                        )));
                    };
                    let Some(db) = state.symbols.read().unwrap().clone() else {
                        return api_result(Err(WebError::InvalidArgument("no ELF loaded".into())));
                    };
                    match db.resolve_name(&name).map(|s| s.id) {
                        Some(i) => i,
                        None => {
                            return api_result(Err(WebError::InvalidArgument(format!(
                                "symbol {name} not found"
                            ))))
                        }
                    }
                }
            };
            crate::watch::WatchTarget::Symbol { symbol_id: resolved_id }
        }
        Some("address") => {
            let address = body.target.get("address").and_then(|v| v.as_u64());
            let Some(address) = address else {
                return api_result(Err(WebError::InvalidArgument("address target needs address".into())));
            };
            crate::watch::WatchTarget::Address { address }
        }
        Some("register") => {
            let name = body.target.get("name").and_then(|v| v.as_str()).map(String::from);
            let Some(name) = name else {
                return api_result(Err(WebError::InvalidArgument("register target needs name".into())));
            };
            crate::watch::WatchTarget::Register { name }
        }
        other => {
            return api_result(Err(WebError::InvalidArgument(format!(
                "target kind must be symbol|address|register, got {other:?}"
            ))))
        }
    };
    let mut items = state.watch.lock().unwrap();
    let id = items.iter().map(|w| w.id).max().unwrap_or(0) + 1;
    items.push(crate::watch::WatchItem {
        id,
        target,
        format: body.format.unwrap_or_else(|| "auto".into()),
        rate_ms: body.rate_ms.unwrap_or(500),
        enabled: true,
    });
    api_result(Ok(json!({ "id": id })))
}

async fn watch_list(State(state): State<SharedState>) -> Response {
    let items = state.watch.lock().unwrap().clone();
    api_result(Ok(json!({ "items": items })))
}

async fn watch_delete(State(state): State<SharedState>, Path(id): Path<u64>) -> Response {
    state.watch.lock().unwrap().retain(|w| w.id != id);
    api_result(Ok(json!({ "deleted": true })))
}

#[derive(Deserialize)]
struct WatchPatchBody {
    enabled: Option<bool>,
    rate_ms: Option<u32>,
}
async fn watch_patch(State(state): State<SharedState>, Path(id): Path<u64>, Json(body): Json<WatchPatchBody>) -> Response {
    let mut items = state.watch.lock().unwrap();
    if let Some(w) = items.iter_mut().find(|w| w.id == id) {
        if let Some(enabled) = body.enabled {
            w.enabled = enabled;
        }
        if let Some(rate_ms) = body.rate_ms {
            w.rate_ms = rate_ms;
        }
        api_result(Ok(json!({ "updated": true })))
    } else {
        api_result(Err(WebError::InvalidArgument(format!("no watch item {id}"))))
    }
}

async fn watch_refresh(State(state): State<SharedState>) -> Response {
    let items = state.watch.lock().unwrap().clone();
    let symbols = state.symbols.read().unwrap().clone();
    let ex = state.executor.clone();
    let mut out = Vec::new();
    for w in &items {
        if !w.enabled {
            continue;
        }
        let read = match &w.target {
            crate::watch::WatchTarget::Address { address } => ex
                .call_async(OperationKind::WatchRead, json!({ "address": address, "width": "u32" }))
                .await
                .ok()
                .and_then(|v| v.get("value").and_then(|x| x.as_u64())),
            crate::watch::WatchTarget::Symbol { symbol_id } => {
                let addr = symbols.as_ref().and_then(|db| db.resolve_id(*symbol_id)).map(|s| s.address);
                match addr {
                    Some(a) => ex
                        .call_async(OperationKind::WatchRead, json!({ "address": a, "width": "u32" }))
                        .await
                        .ok()
                        .and_then(|v| v.get("value").and_then(|x| x.as_u64())),
                    None => None,
                }
            }
            crate::watch::WatchTarget::Register { name } => ex
                .call_async(OperationKind::RegisterRead, json!({ "name": name }))
                .await
                .ok()
                .and_then(|v| v.get("value").and_then(|x| x.as_u64())),
        };
        out.push(json!({ "id": w.id, "value": read }));
    }
    api_result(Ok(json!({ "items": out })))
}

// --- SVD / Peripherals ---

async fn svd_upload(State(state): State<SharedState>, mut multipart: Multipart) -> Response {
    let mut data: Option<Vec<u8>> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name().unwrap_or("") == "file" {
            data = Some(match field.bytes().await {
                Ok(b) => b.to_vec(),
                Err(e) => {
                    return api_result(Err(WebError::Internal(format!("upload read failed: {e}"))))
                }
            });
        }
    }
    let Some(data) = data else {
        return api_result(Err(WebError::InvalidArgument("missing file field".into())));
    };
    let path = state._upload_dir.path().join(format!("{}.svd", uuid::Uuid::new_v4()));
    if let Err(e) = std::fs::write(&path, &data) {
        return api_result(Err(WebError::Internal(format!("failed to save upload: {e}"))));
    }
    let db = match cmsis_dap_core::svd::SvdDatabase::load(&path) {
        Ok(db) => db,
        Err(e) => return api_result(Err(WebError::InvalidArgument(e.to_string()))),
    };
    let summary = db.summary();
    *state.svd.write().unwrap() = Some(db);
    api_result(Ok(json!({ "name": summary.name, "peripherals": summary.peripherals })))
}

async fn peripherals(State(state): State<SharedState>) -> Response {
    let Some(svd) = state.svd.read().unwrap().clone() else {
        return api_result(Err(WebError::InvalidArgument("no SVD loaded".into())));
    };
    let list: Vec<Value> = svd
        .peripherals()
        .iter()
        .map(|p| json!({ "name": p.name, "base": p.base, "registers": p.registers.len() }))
        .collect();
    api_result(Ok(json!({ "peripherals": list })))
}

async fn peripheral_get(State(state): State<SharedState>, Path(name): Path<String>) -> Response {
    let Some(svd) = state.svd.read().unwrap().clone() else {
        return api_result(Err(WebError::InvalidArgument("no SVD loaded".into())));
    };
    match svd.get_peripheral(&name) {
        Some(p) => api_result(Ok(json!({ "peripheral": p }))),
        None => api_result(Err(WebError::InvalidArgument(format!("peripheral {name} not found")))),
    }
}

#[derive(Deserialize)]
struct PeripheralReadBody {
    register: String,
}
async fn peripheral_read(State(state): State<SharedState>, Path(name): Path<String>, Json(body): Json<PeripheralReadBody>) -> Response {
    let Some(svd) = state.svd.read().unwrap().clone() else {
        return api_result(Err(WebError::InvalidArgument("no SVD loaded".into())));
    };
    let (addr, _) = match svd.resolve(&name, &body.register, None) {
        Ok(v) => v,
        Err(e) => return api_result(Err(WebError::InvalidArgument(e.to_string()))),
    };
    let ex = state.executor.clone();
    let result = ex
        .call_async(OperationKind::PeripheralRead, json!({ "address": addr }))
        .await;
    match result {
        Ok(v) => {
            let value = v.get("value").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
            let decoded = svd.decode_register(&name, &body.register, value).ok();
            api_result(Ok(json!({ "address": addr, "value": value, "decoded": decoded })))
        }
        Err(e) => api_result(Err(e)),
    }
}

#[derive(Deserialize)]
struct PeripheralWriteBody {
    register: String,
    value: u32,
}
async fn peripheral_write(State(state): State<SharedState>, Path(name): Path<String>, Json(body): Json<PeripheralWriteBody>) -> Response {
    let Some(svd) = state.svd.read().unwrap().clone() else {
        return api_result(Err(WebError::InvalidArgument("no SVD loaded".into())));
    };
    let (addr, _) = match svd.resolve(&name, &body.register, None) {
        Ok(v) => v,
        Err(e) => return api_result(Err(WebError::InvalidArgument(e.to_string()))),
    };
    api_result(
        state
            .executor
            .clone()
            .call_async(OperationKind::PeripheralWrite, json!({ "address": addr, "value": body.value }))
            .await,
    )
}

#[derive(Deserialize)]
struct PeripheralDecodeBody {
    register: String,
    value: u32,
}
async fn peripheral_decode(State(state): State<SharedState>, Path(name): Path<String>, Json(body): Json<PeripheralDecodeBody>) -> Response {
    let Some(svd) = state.svd.read().unwrap().clone() else {
        return api_result(Err(WebError::InvalidArgument("no SVD loaded".into())));
    };
    match svd.decode_register(&name, &body.register, body.value) {
        Ok(decoded) => api_result(Ok(json!({ "decoded": decoded }))),
        Err(e) => api_result(Err(WebError::InvalidArgument(e.to_string()))),
    }
}
// ---------------------------------------------------------------------------
// Peripheral monitor + RTT / EVR (P5)
// ---------------------------------------------------------------------------

async fn monitor_list(State(state): State<SharedState>) -> Response {
    let items = state.monitors.lock().unwrap().clone();
    api_result(Ok(json!({ "items": items })))
}

#[derive(Deserialize)]
struct MonitorAddBody {
    peripheral: String,
    register: String,
    rate_ms: Option<u32>,
    safety: Option<String>,
}
async fn monitor_add(State(state): State<SharedState>, Json(body): Json<MonitorAddBody>) -> Response {
    let svd = state.svd.read().unwrap().clone();
    let Some(svd) = svd else {
        return api_result(Err(WebError::InvalidArgument("no SVD loaded".into())));
    };
    let reg = match svd.get_register(&body.peripheral, &body.register) {
        Some(r) => r,
        None => {
            return api_result(Err(WebError::InvalidArgument(format!(
                "register {}.{} not found",
                body.peripheral, body.register
            ))))
        }
    };
    // Safety: write-only registers are NotRecommended; everything else starts Unknown.
    let safety = match body.safety.as_deref() {
        Some("safe") => crate::monitor::MonitorSafety::Safe,
        Some("user_confirmed") => crate::monitor::MonitorSafety::UserConfirmed,
        Some("not_recommended") => crate::monitor::MonitorSafety::NotRecommended,
        _ => {
            if reg.access.as_deref() == Some("write-only") {
                crate::monitor::MonitorSafety::NotRecommended
            } else {
                crate::monitor::MonitorSafety::Unknown
            }
        }
    };
    if safety == crate::monitor::MonitorSafety::NotRecommended {
        return api_result(Err(WebError::InvalidArgument(
            "register is write-only; periodic refresh is not recommended".into(),
        )));
    }
    let mut items = state.monitors.lock().unwrap();
    let id = items.iter().map(|m| m.id).max().unwrap_or(0) + 1;
    items.push(crate::monitor::MonitorItem {
        id,
        peripheral: body.peripheral,
        register: body.register,
        rate_ms: body.rate_ms.unwrap_or(500).max(50),
        safety,
    });
    api_result(Ok(json!({ "id": id, "safety": safety })))
}

async fn monitor_delete(State(state): State<SharedState>, Path(id): Path<u64>) -> Response {
    state.monitors.lock().unwrap().retain(|m| m.id != id);
    api_result(Ok(json!({ "deleted": true })))
}

#[derive(Deserialize)]
struct RttStartBody {
    address: Option<u64>,
}
async fn rtt_start(State(state): State<SharedState>, Json(body): Json<RttStartBody>) -> Response {
    let ex = state.executor.clone();
    let result = ex
        .call_async(OperationKind::RttAttach, json!({ "address": body.address }))
        .await;
    let channels = match result {
        Ok(v) => v.get("channels").cloned().unwrap_or(json!([])),
        Err(e) => return api_result(Err(e)),
    };
    // Start the RTT poll task.
    state.rtt_stop.store(false, std::sync::atomic::Ordering::SeqCst);
    let ex2 = state.executor.clone();
    let events = state.events.clone();
    let stop = state.rtt_stop.clone();
    let handle = tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(20));
        loop {
            tick.tick().await;
            if stop.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            let Ok(v) = ex2.call_async(OperationKind::RttRead, json!({ "channels": [], "max_bytes": 4096 })).await else {
                continue;
            };
            let Some(arr) = v.as_array() else { continue };
            for item in arr {
                let channel = item.get("channel").and_then(|c| c.as_u64()).unwrap_or(0) as usize;
                let name = item.get("name").and_then(|n| n.as_str()).map(String::from);
                let data = item.get("data").and_then(|d| d.as_array()).map(|a| a.iter().filter_map(|b| b.as_u64()).map(|b| b as u8).collect::<Vec<u8>>()).unwrap_or_default();
                if data.is_empty() {
                    continue;
                }
                let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                let _ = events.send(ServerEvent::RttOutput { channel, name, data: encoded });
            }
        }
    });
    *state.rtt_task.lock().unwrap() = Some(handle);
    api_result(Ok(json!({ "channels": channels })))
}

async fn rtt_stop(State(state): State<SharedState>) -> Response {
    state.rtt_stop.store(true, std::sync::atomic::Ordering::SeqCst);
    let _ = state
        .executor
        .call_async(OperationKind::RttDetach, json!({}))
        .await;
    api_result(Ok(json!({ "stopped": true })))
}

async fn rtt_channels(State(_state): State<SharedState>) -> Response {
    api_result(Ok(json!({ "channels": [] })))
}

#[derive(Deserialize)]
struct EvrStartBody {
    info_address: u64,
}
async fn evr_start(State(state): State<SharedState>, Json(body): Json<EvrStartBody>) -> Response {
    let ex = state.executor.clone();
    let result = ex
        .call_async(OperationKind::EvrAttach, json!({ "info_address": body.info_address }))
        .await;
    match result {
        Ok(status) => {
            state.evr_stop.store(false, std::sync::atomic::Ordering::SeqCst);
            let ex2 = state.executor.clone();
            let events = state.events.clone();
            let stop = state.evr_stop.clone();
            let handle = tokio::spawn(async move {
                let mut tick = tokio::time::interval(std::time::Duration::from_millis(50));
                loop {
                    tick.tick().await;
                    if stop.load(std::sync::atomic::Ordering::SeqCst) {
                        break;
                    }
                    let Ok(v) = ex2.call_async(OperationKind::EvrRead, json!({})).await else {
                        continue;
                    };
                    let Some(arr) = v.as_array() else { continue };
                    for ev in arr {
                        let _ = events.send(ServerEvent::EvrEventArrived { payload: ev.clone() });
                    }
                }
            });
            *state.evr_task.lock().unwrap() = Some(handle);
            api_result(Ok(json!({ "status": status })))
        }
        Err(e) => api_result(Err(e)),
    }
}

async fn evr_stop(State(state): State<SharedState>) -> Response {
    state.evr_stop.store(true, std::sync::atomic::Ordering::SeqCst);
    let _ = state
        .executor
        .call_async(OperationKind::EvrDetach, json!({}))
        .await;
    api_result(Ok(json!({ "stopped": true })))
}
// ---------------------------------------------------------------------------
// Disassembly + address describe (P5)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct DisassemblyQuery {
    address: u64,
    count: Option<usize>,
}
async fn disassembly(State(state): State<SharedState>, Query(q): Query<DisassemblyQuery>) -> Response {
    let count = q.count.unwrap_or(16).clamp(1, 128);
    let bytes = match state
        .executor
        .clone()
        .call_async(
            OperationKind::MemoryRead,
            json!({ "address": q.address, "width": "u8", "count": count * 4 }),
        )
        .await
    {
        Ok(v) => v.get("bytes").and_then(|b| b.as_array()).map(|a| a.iter().filter_map(|x| x.as_u64()).map(|x| x as u8).collect::<Vec<u8>>()).unwrap_or_default(),
        Err(e) => return api_result(Err(e)),
    };
    let cs = match capstone::Capstone::new()
        .arm()
        .mode(capstone::arch::arm::ArchMode::Thumb)
        .extra_mode(std::iter::once(capstone::arch::arm::ArchExtraMode::MClass))
        .detail(true)
        .build()
    {
        Ok(cs) => cs,
        Err(e) => return api_result(Err(WebError::Internal(format!("capstone init: {e}")))),
    };
    let symbols = state.symbols.read().unwrap().clone();
    let debug_info = state.debug_info.lock().unwrap();
    let disassembled = match cs.disasm_all(&bytes, q.address) {
        Ok(d) => d,
        Err(e) => return api_result(Err(WebError::Internal(e.to_string()))),
    };
    let mut instructions = Vec::new();
    for insn in disassembled.iter() {
        if instructions.len() >= count {
            break;
        }
        let addr = insn.address();
        let sym = symbols
            .as_ref()
            .and_then(|db| db.resolve_address(addr))
            .map(|(s, off)| format!("{} + 0x{:x}", s.name, off));
        let source = debug_info
            .as_ref()
            .and_then(|di| di.find_location(addr))
            .map(|sl| json!({ "file": sl.file, "line": sl.line }));
        instructions.push(json!({
            "address": addr,
            "bytes": insn.bytes().iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" "),
            "mnemonic": insn.mnemonic().unwrap_or("?"),
            "op_str": insn.op_str().unwrap_or(""),
            "symbol": sym,
            "source": source,
            "pc": addr,
        }));
    }
    api_result(Ok(json!({ "address": q.address, "instructions": instructions })))
}

async fn address_describe(State(state): State<SharedState>, Path(address): Path<u64>) -> Response {
    let symbols = state.symbols.read().unwrap().clone();
    let svd = state.svd.read().unwrap().clone();
    let regions = match state
        .executor
        .call_async(OperationKind::TargetInfo, json!({}))
        .await
    {
        Ok(v) => v
            .get("target")
            .and_then(|t| t.get("memory_regions"))
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| {
                        let name = r.get("name").and_then(|n| n.as_str())?.to_string();
                        let start = r.get("start").and_then(|s| s.as_u64())?;
                        let end = r.get("end").and_then(|e| e.as_u64())?;
                        Some((name, start, end))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    let mut info = crate::resolver::describe_address(
        symbols.as_ref().unwrap_or(&cmsis_dap_core::symbols::SymbolDatabase::new()),
        svd.as_ref(),
        &regions,
        address,
    );
    let source = state
        .debug_info
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|di| di.find_location(address));
    if let Some(src) = source {
        info.symbol = info.symbol.or_else(|| {
            src.function.as_ref().map(|f| crate::resolver::SymbolAt {
                name: f.clone(),
                address,
                offset: 0,
            })
        });
        api_result(Ok(json!({ "address": info, "source_location": src })))
    } else {
        api_result(Ok(json!({ "address": info })))
    }
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



