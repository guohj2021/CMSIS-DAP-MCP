use cmsis_dap_core::backend::mock::MockBackend;
use cmsis_dap_core::backend::{AccessWidth, ConnectOptions, CoreRegister, Protocol};
use cmsis_dap_core::remote;
use cmsis_dap_core::session::SessionManager;
use std::sync::{Arc, Mutex};

fn connect(mock: MockBackend) -> SessionManager {
    let mut session = SessionManager::new(Box::new(mock));
    session
        .connect(&ConnectOptions {
            probe_id: None,
            protocol: Protocol::Swd,
            speed_khz: None,
            target: None,
            under_reset: false,
            core_index: None,
        })
        .unwrap();
    session
}

#[test]
fn dump_restores_running_core_and_collects_fields() {
    let mut session = connect(MockBackend::new());
    session
        .backend()
        .write_core_register(&CoreRegister::Name("pc".into()), 0x0800_0100)
        .unwrap();
    session
        .backend()
        .write_core_register(&CoreRegister::Name("msp".into()), 0x2000_2000)
        .unwrap();
    session
        .backend()
        .write_memory(0x2000_2000, AccessWidth::U32, &[0x1111_1111])
        .unwrap();
    session
        .backend()
        .write_memory(0x2000_0000, AccessWidth::U32, &[0xDEAD_BEEF])
        .unwrap();

    let dump = session
        .backend()
        .dump_cpu_state(&[0x2000_0000], 4, true)
        .unwrap();
    assert_eq!(dump.state, "running");
    assert_eq!(dump.pc, Some(0x0800_0100));
    assert!(dump
        .registers
        .iter()
        .any(|r| r.name == "pc" && r.value == 0x0800_0100));
    assert!(dump
        .memory
        .iter()
        .any(|m| m.address == 0x2000_0000 && m.value == 0xDEAD_BEEF));
    assert_eq!(dump.stack_msp[0], 0x1111_1111);
    assert_eq!(
        session.backend().get_core_status().unwrap().state,
        "running",
        "running core should be restored after the dump"
    );
}

#[test]
fn dump_without_restore_leaves_core_halted() {
    let mut session = connect(MockBackend::new());
    session.backend().dump_cpu_state(&[], 0, false).unwrap();
    assert_eq!(session.backend().get_core_status().unwrap().state, "halted");
}

#[test]
fn dump_halted_core_stays_halted() {
    let mut session = connect(MockBackend::new());
    session.backend().halt().unwrap();
    let dump = session.backend().dump_cpu_state(&[], 0, true).unwrap();
    assert_eq!(dump.state, "halted");
    assert!(dump.halt_reason.is_some());
    assert_eq!(session.backend().get_core_status().unwrap().state, "halted");
}

#[test]
fn dump_includes_fault_registers() {
    let mut session = connect(MockBackend::new());
    let dump = session.backend().dump_cpu_state(&[], 0, true).unwrap();
    for name in ["CFSR", "HFSR", "DFSR", "MMFAR", "BFAR"] {
        assert!(
            dump.fault.iter().any(|f| f.name == name),
            "missing fault register {name}"
        );
    }
}

#[tokio::test]
async fn remote_tcp_json_rpc_roundtrip() {
    let mut session = connect(MockBackend::new());
    session
        .backend()
        .write_memory(0x2000_0000, AccessWidth::U32, &[0xDEAD_BEEF])
        .unwrap();
    let shared = Arc::new(Mutex::new(session));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        remote::serve_listener(listener, &shared).await.unwrap();
    });

    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap();
    let (reader_half, mut writer_half) = stream.into_split();
    writer_half
        .write_all(b"{\"id\":1,\"method\":\"read_memory\",\"params\":{\"address\":536870912,\"width\":\"u32\",\"count\":1}}\n")
        .await
        .unwrap();
    let mut reader = BufReader::new(reader_half);
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    let response: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(response["id"], 1);
    assert_eq!(response["result"]["values"][0].as_u64(), Some(0xDEAD_BEEF));

    // dump_cpu_state over TCP (non-invasive, restores run state).
    writer_half
        .write_all(b"{\"id\":2,\"method\":\"dump_cpu_state\",\"params\":{\"addresses\":[536870912],\"stack_words\":4}}\n")
        .await
        .unwrap();
    line.clear();
    reader.read_line(&mut line).await.unwrap();
    let response: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(response["id"], 2);
    assert_eq!(response["result"]["state"], "running");
    assert_eq!(
        response["result"]["memory"][0]["value"].as_u64(),
        Some(0xDEAD_BEEF)
    );

    // Unknown method returns an error payload.
    writer_half
        .write_all(b"{\"id\":3,\"method\":\"nope\",\"params\":{}}\n")
        .await
        .unwrap();
    line.clear();
    reader.read_line(&mut line).await.unwrap();
    let response: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(response["id"], 3);
    assert!(response["error"].is_object());

    drop(writer_half);
    server.abort();
}
