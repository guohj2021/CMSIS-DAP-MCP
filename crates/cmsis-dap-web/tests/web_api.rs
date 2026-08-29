//! Integration tests: REST + executor + three-layer state over MockBackend.
//!
//! These exercise the real axum router and the DebugExecutor (the single
//! hardware I/O owner) without touching hardware.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use cmsis_dap_core::backend::mock::MockBackend;
use cmsis_dap_core::session::SessionManager;
use cmsis_dap_web::executor::{spawn, ExecutorConfig};
use cmsis_dap_web::op::{OperationKind, ServerEvent, ServerState, TargetState};
use cmsis_dap_web::state::{AppState, SharedState};
use cmsis_dap_web::WebServerOptions;
use http_body_util::BodyExt;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;

fn app() -> (axum::Router, SharedState) {
    app_with_destructive(false)
}

fn app_with_destructive(allow_destructive: bool) -> (axum::Router, SharedState) {
    let session = SessionManager::new(Box::new(MockBackend::new()));
    let executor = spawn(
        session,
        ExecutorConfig {
            allow_destructive,
            flash_timeout: Duration::from_secs(600),
        },
    );
    let events = executor.events();
    let lease = cmsis_dap_web::session::SessionLease::new(Duration::from_secs(30));
    let upload_dir = tempfile::Builder::new().prefix("cmsis-dap-web-test-").tempdir().unwrap();
    let state = Arc::new(AppState {
        executor,
        events,
        lease,
        host: "127.0.0.1".into(),
        port: 0,
        default_connect: WebServerOptions::default().default_connect,
        uploads: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        _upload_dir: upload_dir,
    });
    (cmsis_dap_web::build_router(state.clone()), state)
}

async fn send(app: &axum::Router, method: &str, path: &str, body: Option<serde_json::Value>) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(match body {
            Some(b) => Body::from(b.to_string()),
            None => Body::empty(),
        })
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::json!({}));
    (status, json)
}

#[tokio::test]
async fn health_reports_disconnected() {
    let (app, _) = app();
    let (status, json) = send(&app, "GET", "/api/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["state"]["server"], "disconnected");
}

#[tokio::test]
async fn connect_run_halt_reset_state_machine() {
    let (app, state) = app();
    // Not connected -> status rejected.
    let (status, json) = send(&app, "GET", "/api/status", None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(json["error"]["code"], "not_connected");

    // Connect.
    let (status, json) = send(&app, "POST", "/api/connect", Some(serde_json::json!({}))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["target"]["core_type"], "Cortex-M0");
    assert_eq!(state.executor.status().server, ServerState::Ready);

    // Run.
    let (status, _) = send(&app, "POST", "/api/debug/run", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(state.executor.status().target, TargetState::Running);

    // Halt.
    let (status, _) = send(&app, "POST", "/api/debug/halt", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(state.executor.status().target, TargetState::Halted);

    // Reset & Halt.
    let (status, json) = send(&app, "POST", "/api/debug/reset", Some(serde_json::json!({ "mode": "halt" }))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["mode"], "halt");
    assert_eq!(state.executor.status().target, TargetState::Halted);
}

#[tokio::test]
async fn registers_and_memory_roundtrip() {
    let (app, _) = app();
    send(&app, "POST", "/api/connect", Some(serde_json::json!({}))).await;

    let (status, json) = send(&app, "GET", "/api/registers", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!json["registers"].as_array().unwrap().is_empty());

    // Memory write then read back.
    let (status, _) = send(
        &app,
        "POST",
        "/api/memory/write",
        Some(serde_json::json!({ "address": 0x20000000, "width": "u32", "values": [0x11223344u64, 2864434397u64] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, json) = send(
        &app,
        "POST",
        "/api/memory/read",
        Some(serde_json::json!({ "address": 0x20000000, "width": "u32", "count": 2 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let bytes = json["bytes"].as_array().unwrap();
    assert_eq!(bytes.len(), 8);
    assert_eq!(bytes[0], 0x44);
    assert_eq!(bytes[3], 0x11);
    assert_eq!(bytes[4], 0xDD);
    assert_eq!(bytes[7], 0xAA);
}

#[tokio::test]
async fn breakpoints_and_watchpoints() {
    let (app, state) = app();
    send(&app, "POST", "/api/connect", Some(serde_json::json!({}))).await;

    // Breakpoint set/list/limits.
    let (status, _) = send(&app, "POST", "/api/breakpoints", Some(serde_json::json!({ "address": 0x08000100 }))).await;
    assert_eq!(status, StatusCode::OK);
    let (status, json) = send(&app, "GET", "/api/breakpoints", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["breakpoints"][0], 0x08000100);

    let (status, json) = send(&app, "GET", "/api/breakpoints/limits", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["used"], 1);
    assert_eq!(json["total"], 4);

    // Watchpoint set/list.
    let (status, _) = send(
        &app,
        "POST",
        "/api/watchpoints",
        Some(serde_json::json!({ "address": 0x20000010, "access": "rw" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, json) = send(&app, "GET", "/api/watchpoints", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["watchpoints"][0]["address"], 0x20000010);

    let _ = state;
}

#[tokio::test]
async fn destructive_gate_blocks_flash_software_breakpoint() {
    let (app, _) = app();
    send(&app, "POST", "/api/connect", Some(serde_json::json!({}))).await;

    let (status, json) = send(
        &app,
        "POST",
        "/api/breakpoints",
        Some(serde_json::json!({ "address": 0x08000100, "kind": "sw_flash" })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(json["error"]["code"], "destructive_disabled");
}

#[tokio::test]
async fn ws_event_flow_ready_and_state_changed() {
    // Verify the executor emits TargetStateChanged events via broadcast.
    let (_, state) = app();
    let mut rx = state.events.subscribe();
    let ex = state.executor.clone();

    ex.call_async(OperationKind::Connect, serde_json::json!({}))
        .await
        .unwrap();

    let mut saw_ready = false;
    for _ in 0..8 {
        if let Ok(ServerEvent::TargetStateChanged { .. }) = rx.recv().await {
            if ex.status().server == ServerState::Ready {
                saw_ready = true;
            }
        }
        if saw_ready {
            break;
        }
    }
    assert!(saw_ready, "expected a ready server state event after connect");
}
#[tokio::test]
async fn firmware_upload_hex_analyze_and_region_validation() {
    let (app, state) = app_with_destructive(true);
    send(&app, "POST", "/api/connect", Some(serde_json::json!({}))).await;

    // Mock flash region is 0x0800_0000..0x0801_0000 (nvm).
    // Write a tiny HEX image at 0x0800_0000.
    let hex = ":020000040800F2\n:040000001122334452\n:00000001FF\n";
    let id = "test-hex".to_string();
    let path = state._upload_dir.path().join("test.hex");
    std::fs::write(&path, hex).unwrap();
    state.uploads.lock().unwrap().insert(id.clone(), path);

    // Analyze via the upload path is exercised through Multipart; here we
    // validate the program pipeline directly.
    let upload_path = state.uploads.lock().unwrap().get(&id).unwrap().clone();
    let result = cmsis_dap_web::flash::program_image(
        &state.executor,
        &id,
        None,
        true,
        "stay_halted",
        cmsis_dap_web::flash::FirmwareFormat::Hex,
        &upload_path,
    )
    .await;
    assert!(result.is_ok(), "program_image failed: {result:?}");
    let json = result.unwrap();
    assert_eq!(json["programmed"], true);
    assert!(json["bytes"].as_u64().unwrap() > 0, "programmed some bytes");
}

#[tokio::test]
async fn flash_region_validation_rejects_out_of_range() {
    let (app, state) = app();
    send(&app, "POST", "/api/connect", Some(serde_json::json!({}))).await;

    // HEX segment far outside flash (0x4000_0000) must fail validation.
    let hex = ":020000044000BA\n:040000001122334452\n:00000001FF\n";
    let path = state._upload_dir.path().join("bad.hex");
    std::fs::write(&path, hex).unwrap();
    state.uploads.lock().unwrap().insert("bad".into(), path);

    let upload_path = state.uploads.lock().unwrap().get("bad").unwrap().clone();
    let result = cmsis_dap_web::flash::program_image(
        &state.executor,
        "bad",
        None,
        true,
        "stay_halted",
        cmsis_dap_web::flash::FirmwareFormat::Hex,
        &upload_path,
    )
    .await;
    match result {
        Err(cmsis_dap_web::op::WebError::RegionOverflow(_)) => {}
        other => panic!("expected RegionOverflow, got {other:?}"),
    }
}