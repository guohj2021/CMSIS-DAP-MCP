use cmsis_dap_core::backend::mock::MockBackend;
use cmsis_dap_core::backend::{AccessWidth, Backend, ConnectOptions, Protocol};
use cmsis_dap_core::evr;

const INFO_VALID: u32 = 0x0800_0000;
const INFO_LOCKED: u32 = 0x0400_0000;

fn record(ts: u32, val1: u32, val2: u32, info: u32) -> [u8; 16] {
    let mut r = [0u8; 16];
    r[0..4].copy_from_slice(&ts.to_le_bytes());
    r[4..8].copy_from_slice(&val1.to_le_bytes());
    r[8..12].copy_from_slice(&val2.to_le_bytes());
    r[12..16].copy_from_slice(&info.to_le_bytes());
    r
}

fn info(message: u8, component: u8, context: u8, extra: u32) -> u32 {
    (message as u32) | ((component as u32) << 8) | ((context as u32) << 16) | extra | INFO_VALID
}

#[test]
fn decodes_valid_record_fields() {
    let raw = record(
        0x0000_1000,
        0x1111_1111,
        0x2222_2222,
        info(0x05, 0xFE, 2, 0x0040_0000), // context 2, seq=4
    );
    let event = evr::decode_record(&raw, 0, 1_000_000).expect("valid record");
    assert_eq!(event.timestamp_ticks, 0x1000);
    assert!((event.timestamp_secs - 0.004096).abs() < 1e-9);
    assert_eq!(event.context, 2);
    assert_eq!(event.component, 0xFE);
    assert_eq!(event.message, 0x05);
    assert_eq!(event.sequence, 4);
    assert_eq!(event.val1, 0x1111_1111);
    assert_eq!(event.val2, 0x2222_2222);
    assert!(!event.irq);
    assert!(!event.first);
    assert!(!event.last);
}

#[test]
fn rejects_locked_and_invalid_records() {
    let locked = record(0, 0, 0, info(1, 1, 0, 0) | INFO_LOCKED);
    assert!(evr::decode_record(&locked, 0, 0).is_none());

    let not_valid = record(0, 0, 0, 0x1234);
    assert!(evr::decode_record(&not_valid, 0, 0).is_none());

    let short = [0u8; 8];
    assert!(evr::decode_record(&short, 0, 0).is_none());
}

#[test]
fn reconstructs_msb_and_overflow() {
    // Real timestamp has bit 31 set: carried in info bit 28, cleared in the
    // stored 32-bit word (the stored MSB is the toggle bit).
    let ts = 0x8000_1234u32;
    let raw = record(ts & 0x7FFF_FFFF, 0, 0, info(0, 1, 1, 0) | evr::INFO_MSB_TS);
    let event = evr::decode_record(&raw, 3, 0).expect("valid record");
    assert_eq!(event.timestamp_ticks, (3u64 << 32) | (ts as u64));
    assert_eq!(event.context, 1);
}

#[test]
fn plan_returns_new_indices_and_advances() {
    let header = evr::EvrInfoHeader {
        protocol_version: "1.1".into(),
        record_count: 8,
        event_buffer: 0x2000_0000,
        event_status: 0x2000_0100,
        ts_source: 0,
    };
    let mut reader = evr::EvrReader::new(&header);
    let status = evr::EvrStatusFields {
        state: 1,
        record_index: 3,
        records_written: 3,
        records_dumped: 0,
        ts_overflow: 0,
        ts_freq: 1_000_000,
        init_count: 1,
        signature: 0,
    };
    assert_eq!(reader.plan(&status), vec![0, 1, 2]);
    for i in [0u32, 1, 2] {
        reader.advance(i);
    }
    assert_eq!(reader.last_index, 3);

    // No new records.
    assert!(reader.plan(&status).is_empty());
}

#[test]
fn plan_handles_index_wraparound() {
    let header = evr::EvrInfoHeader {
        protocol_version: "1.1".into(),
        record_count: 8,
        event_buffer: 0,
        event_status: 0,
        ts_source: 0,
    };
    let mut reader = evr::EvrReader::new(&header);
    reader.last_index = 0xFFFF_FFFE;
    let status = evr::EvrStatusFields {
        record_index: 2,
        ..default_status()
    };
    let indices = reader.plan(&status);
    assert_eq!(indices, vec![0xFFFF_FFFE, 0xFFFF_FFFF, 0, 1]);
}

#[test]
fn plan_clamps_when_records_were_overwritten() {
    let header = evr::EvrInfoHeader {
        protocol_version: "1.1".into(),
        record_count: 8,
        event_buffer: 0,
        event_status: 0,
        ts_source: 0,
    };
    let mut reader = evr::EvrReader::new(&header);
    let status = evr::EvrStatusFields {
        record_index: 20,
        ..default_status()
    };
    let indices = reader.plan(&status);
    assert_eq!(indices.len(), 8);
    assert_eq!(indices[0], 12);
    assert_eq!(indices[7], 19);
    assert_eq!(reader.last_index, 12);
}

fn default_status() -> evr::EvrStatusFields {
    evr::EvrStatusFields {
        state: 1,
        record_index: 0,
        records_written: 0,
        records_dumped: 0,
        ts_overflow: 0,
        ts_freq: 1_000_000,
        init_count: 1,
        signature: 0,
    }
}

fn connect(mock: &mut MockBackend) {
    mock.connect(&ConnectOptions {
        probe_id: None,
        protocol: Protocol::Swd,
        speed_khz: None,
        target: None,
        under_reset: false,
        core_index: None,
    })
    .unwrap();
}

#[test]
fn mock_rtt_reads_and_drains_channels() {
    let mut mock = MockBackend::with_rtt(&[(Some("up0"), b"hello"), (None, b"world")]);
    connect(&mut mock);
    let channels = mock.attach_rtt(None).unwrap();
    assert_eq!(channels.len(), 2);
    assert_eq!(channels[0].name.as_deref(), Some("up0"));
    assert_eq!(channels[1].name, None);

    let reads = mock.read_rtt(&[0, 1], 16).unwrap();
    assert_eq!(reads.len(), 2);
    assert_eq!(reads[0].data, b"hello");
    assert_eq!(reads[1].data, b"world");

    // Drained: a second read yields nothing.
    let reads = mock.read_rtt(&[0, 1], 16).unwrap();
    assert!(reads.is_empty());

    mock.detach_rtt().unwrap();
    assert!(mock.read_rtt(&[0], 16).is_err());
}

#[test]
fn mock_evr_reads_committed_records() {
    let events = vec![
        record(0x1000, 1, 2, info(0x01, 0xFE, 0, 0)),
        record(0x2000, 3, 4, info(0x02, 0x03, 3, 0)),
    ];
    let mut mock = MockBackend::with_evr(8, 1_000_000, events);
    connect(&mut mock);
    let status = mock.attach_evr(0x2000_0200).unwrap();
    assert_eq!(status.record_count, 8);
    assert_eq!(status.records_written, 2);
    assert_eq!(status.ts_freq, 1_000_000);

    let decoded = mock.read_evr().unwrap();
    assert_eq!(decoded.len(), 2);
    assert_eq!(decoded[0].context, 0);
    assert_eq!(decoded[1].context, 3);
    assert_eq!(decoded[1].timestamp_ticks, 0x2000);

    // All records consumed.
    assert!(mock.read_evr().unwrap().is_empty());
    mock.detach_evr().unwrap();
    assert!(mock.read_evr().is_err());
}

#[test]
fn mock_evr_requires_connection() {
    let mut mock = MockBackend::new();
    assert!(mock.attach_evr(0).is_err());
    connect(&mut mock);
    mock.write_memory(0x2000_0000, AccessWidth::U32, &[0x1234_5678])
        .unwrap();
    assert_eq!(
        mock.read_memory(0x2000_0000, AccessWidth::U32, 1).unwrap()[0],
        0x1234_5678
    );
}
