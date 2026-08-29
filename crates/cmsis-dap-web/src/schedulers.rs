//! Schedulers for live debug (frozen v5 §12 / P5): Live Watch and Peripheral
//! periodic refresh. All target I/O funnels through the executor.

use crate::executor::ExecutorHandle;
use crate::op::{OperationKind, ServerEvent, ServerState};
use crate::state::SharedState;
use crate::watch::WatchTarget;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Start the Live Watch scheduler. Reads enabled watch items grouped by their
/// refresh rate and pushes `live_watch_value_changed` events (incremental).
pub fn spawn_live_watch(state: SharedState, stop: Arc<AtomicBool>) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_millis(50));
        // item_id -> next due instant (ms counter)
        let mut due: std::collections::HashMap<u64, u128> = std::collections::HashMap::new();
        let mut now_ms = 0u128;
        loop {
            tick.tick().await;
            now_ms += 50;
            if stop.load(Ordering::SeqCst) {
                break;
            }
            if state.executor.status().server != ServerState::Ready {
                continue;
            }
            let items = state.watch.lock().unwrap().clone();
            let mut to_read: Vec<(u64, WatchTarget)> = Vec::new();
            for w in &items {
                if !w.enabled {
                    continue;
                }
                let rate = (w.rate_ms.max(50)) as u128;
                let d = due.entry(w.id).or_insert(0);
                if now_ms >= *d {
                    *d = now_ms + rate;
                    to_read.push((w.id, w.target.clone()));
                }
            }
            if to_read.is_empty() {
                continue;
            }
            let symbols = state.symbols.read().unwrap().clone();
            let mut out = Vec::new();
            for (id, target) in to_read {
                let value = read_target(&state.executor, &symbols, &target).await;
                out.push(serde_json::json!({ "id": id, "value": value }));
            }
            if !out.is_empty() {
                let _ = state
                    .events
                    .send(ServerEvent::LiveWatchValueChanged { items: out });
            }
        }
    });
}

/// Start the Peripheral periodic monitor. Only monitored registers are read.
pub fn spawn_peripheral_monitor(state: SharedState, stop: Arc<AtomicBool>) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_millis(50));
        let mut due: std::collections::HashMap<u64, u128> = std::collections::HashMap::new();
        let mut now_ms = 0u128;
        loop {
            tick.tick().await;
            now_ms += 50;
            if stop.load(Ordering::SeqCst) {
                break;
            }
            if state.executor.status().server != ServerState::Ready {
                continue;
            }
            let monitors = state.monitors.lock().unwrap().clone();
            let svd = state.svd.read().unwrap().clone();
            let mut to_read: Vec<(u64, String, String, u64)> = Vec::new();
            for m in &monitors {
                let rate = (m.rate_ms.max(50)) as u128;
                let d = due.entry(m.id).or_insert(0);
                if now_ms >= *d {
                    *d = now_ms + rate;
                    let addr = svd
                        .as_ref()
                        .and_then(|s| s.resolve(&m.peripheral, &m.register, None).ok())
                        .map(|(a, _)| a);
                    if let Some(a) = addr {
                        to_read.push((m.id, m.peripheral.clone(), m.register.clone(), a));
                    }
                }
            }
            if to_read.is_empty() {
                continue;
            }
            let mut out = Vec::new();
            for (id, periph, reg, addr) in to_read {
                let value = state
                    .executor
                    .call_async(OperationKind::PeripheralRead, serde_json::json!({ "address": addr }))
                    .await
                    .ok()
                    .and_then(|v| v.get("value").and_then(|x| x.as_u64()));
                out.push(serde_json::json!({ "id": id, "peripheral": periph, "register": reg, "value": value }));
            }
            if !out.is_empty() {
                let _ = state
                    .events
                    .send(ServerEvent::PeripheralValueChanged { items: out });
            }
        }
    });
}

async fn read_target(
    executor: &ExecutorHandle,
    symbols: &Option<cmsis_dap_core::symbols::SymbolDatabase>,
    target: &WatchTarget,
) -> Option<u64> {
    match target {
        WatchTarget::Address { address } => executor
            .call_async(OperationKind::WatchRead, serde_json::json!({ "address": address, "width": "u32" }))
            .await
            .ok()
            .and_then(|v| v.get("value").and_then(|x| x.as_u64())),
        WatchTarget::Symbol { symbol_id } => {
            let addr = symbols.as_ref().and_then(|db| db.resolve_id(*symbol_id)).map(|s| s.address);
            match addr {
                Some(a) => executor
                    .call_async(OperationKind::WatchRead, serde_json::json!({ "address": a, "width": "u32" }))
                    .await
                    .ok()
                    .and_then(|v| v.get("value").and_then(|x| x.as_u64())),
                None => None,
            }
        }
        WatchTarget::Register { name } => executor
            .call_async(OperationKind::RegisterRead, serde_json::json!({ "name": name }))
            .await
            .ok()
            .and_then(|v| v.get("value").and_then(|x| x.as_u64())),
    }
}
