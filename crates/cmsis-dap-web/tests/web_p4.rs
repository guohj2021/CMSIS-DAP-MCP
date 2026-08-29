//! P4 integration tests: ELF symbols, watch, SVD peripherals over the API.
use axum::body::Body;
use axum::http::{Request, StatusCode};
use cmsis_dap_core::backend::mock::MockBackend;
use cmsis_dap_core::session::SessionManager;
use cmsis_dap_web::executor::{spawn, ExecutorConfig};
use cmsis_dap_web::state::{AppState, SharedState};
use cmsis_dap_web::WebServerOptions;
use http_body_util::BodyExt;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;

fn app() -> (axum::Router, SharedState) {
    let session = SessionManager::new(Box::new(MockBackend::new()));
    let executor = spawn(
        session,
        ExecutorConfig { allow_destructive: true, flash_timeout: Duration::from_secs(600) },
    );
    let events = executor.events();
    let lease = cmsis_dap_web::session::SessionLease::new(Duration::from_secs(30));
    let upload_dir = tempfile::Builder::new().prefix("cmsis-dap-web-p4-").tempdir().unwrap();
    let state = Arc::new(AppState {
        executor,
        events,
        lease,
        host: "127.0.0.1".into(),
        port: 0,
        default_connect: WebServerOptions::default().default_connect,
        uploads: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        _upload_dir: upload_dir,
        symbols: Arc::new(std::sync::RwLock::new(None)),
        debug_info: Arc::new(std::sync::Mutex::new(None)),
        svd: Arc::new(std::sync::RwLock::new(None)),
        watch: Arc::new(std::sync::Mutex::new(Vec::new())),
        monitors: Arc::new(std::sync::Mutex::new(Vec::new())),
        scheduler_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        rtt_task: Arc::new(std::sync::Mutex::new(None)),
        evr_task: Arc::new(std::sync::Mutex::new(None)),
        rtt_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        evr_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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

fn make_elf() -> Vec<u8> {
    use object::write::{Object, Symbol as WSymbol, SymbolSection, SymbolScope};
    use object::{Architecture, BinaryFormat, Endianness, SymbolFlags, SymbolKind as ObjKind};
    let mut obj = Object::new(BinaryFormat::Elf, Architecture::Arm, Endianness::Little);
    obj.add_file_symbol("main.c".as_bytes().to_vec());
    let text = obj.add_section(Vec::new(), b".text".to_vec(), object::SectionKind::Text);
    let data = obj.add_section(Vec::new(), b".data".to_vec(), object::SectionKind::Data);
    obj.append_section_data(text, &[0x00; 64], 4);
    obj.append_section_data(data, &[0x11; 16], 4);
    obj.add_symbol(WSymbol {
        name: b"main".to_vec(), value: 0x0800_0100, size: 64, kind: ObjKind::Text,
        scope: SymbolScope::Dynamic, weak: false, section: SymbolSection::Section(text), flags: SymbolFlags::None,
    });
    obj.add_symbol(WSymbol {
        name: b"g_motor_speed".to_vec(), value: 0x2000_0000, size: 4, kind: ObjKind::Data,
        scope: SymbolScope::Dynamic, weak: false, section: SymbolSection::Section(data), flags: SymbolFlags::None,
    });
    let mut buf = Vec::new();
    obj.write_stream(&mut buf).unwrap();
    buf
}

fn make_svd() -> Vec<u8> {
    r#"<?xml version="1.0"?>
<device schemaVersion="1.1">
<vendor>Test</vendor><name>Mini</name><version>1.0</version><description>Mini</description>
<addressUnitBits>8</addressUnitBits><width>32</width><size>32</size><access>read-write</access>
<resetValue>0x00000000</resetValue><resetMask>0xFFFFFFFF</resetMask>
<peripherals>
<peripheral><name>GPIOA</name><description>GPIO A</description><baseAddress>0x48000000</baseAddress>
<addressBlock><offset>0x0</offset><size>0x400</size><usage>registers</usage></addressBlock>
<registers><register><name>ODR</name><description>output data</description><addressOffset>0x14</addressOffset>
<size>32</size><access>read-write</access><resetValue>0x0</resetValue>
<fields><field><name>ODR0</name><bitOffset>0</bitOffset><bitWidth>1</bitWidth></field></fields>
</register></registers></peripheral>
</peripherals></device>"#
    .as_bytes()
    .to_vec()
}

#[tokio::test]
async fn elf_upload_symbols_search_and_watch() {
    let (app, state) = app();
    send(&app, "POST", "/api/connect", Some(serde_json::json!({}))).await;

    // Upload ELF via multipart.
    let elf = make_elf();
    let boundary = "----testboundary42";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"demo.axf\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(&elf);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let req = Request::builder()
        .method("POST")
        .uri("/api/files/elf")
        .header("content-type", format!("multipart/form-data; boundary={boundary}"))
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let upload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(upload["functions"].as_u64().unwrap() >= 1);
    assert!(upload["variables"].as_u64().unwrap() >= 1);

    // Search symbols.
    let (status, json) = send(&app, "GET", "/api/symbols?pattern=motor&limit=10", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["total"], 1);
    assert_eq!(json["items"][0]["name"], "g_motor_speed");

    // Add watch by symbol name and refresh.
    let (status, json) = send(
        &app,
        "POST",
        "/api/watch",
        Some(serde_json::json!({ "target": { "kind": "symbol", "name": "g_motor_speed" } })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let id = json["id"].as_u64().unwrap();

    let (status, json) = send(&app, "POST", "/api/watch/refresh", None).await;
    assert_eq!(status, StatusCode::OK);
    let items = json["items"].as_array().unwrap();
    assert_eq!(items[0]["id"], id);
    assert!(items[0]["value"].is_u64());

    // Breakpoint by symbol.
    let (status, _) = send(
        &app,
        "POST",
        "/api/breakpoints",
        Some(serde_json::json!({ "target": { "kind": "symbol", "name": "main" } })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let _ = state;
}

#[tokio::test]
async fn svd_upload_peripheral_read_and_decode() {
    let (app, state) = app();
    send(&app, "POST", "/api/connect", Some(serde_json::json!({}))).await;

    let svd = make_svd();
    let boundary = "----svdboundary7";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"mini.svd\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(&svd);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let req = Request::builder()
        .method("POST")
        .uri("/api/svd")
        .header("content-type", format!("multipart/form-data; boundary={boundary}"))
        .body(Body::from(body))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let (status, json) = send(&app, "GET", "/api/peripherals", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["peripherals"][0]["name"], "GPIOA");

    // Decode without touching hardware.
    let (status, json) = send(
        &app,
        "POST",
        "/api/peripherals/GPIOA/decode",
        Some(serde_json::json!({ "register": "ODR", "value": 1 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["decoded"]["fields"][0]["name"], "ODR0");

    let _ = state;
}
