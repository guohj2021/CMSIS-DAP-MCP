//! Browser-session lease (v5 §6).
//!
//! Browser disconnect (WS close / tab refresh) is distinct from probe
//! disconnect: the probe session is kept for a grace period so a refresh can
//! resume. Probe loss is handled by the executor (immediate invalidation).

use crate::executor::ExecutorHandle;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct SessionLease {
    clients: AtomicUsize,
    grace: Duration,
}

impl SessionLease {
    pub fn new(grace: Duration) -> Arc<Self> {
        Arc::new(Self {
            clients: AtomicUsize::new(0),
            grace,
        })
    }

    pub fn client_connected(&self) {
        self.clients.fetch_add(1, Ordering::SeqCst);
    }

    pub fn client_disconnected(&self) {
        self.clients.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn active_clients(&self) -> usize {
        self.clients.load(Ordering::SeqCst)
    }

    /// Schedule a grace-period check: when no browser clients remain after
    /// `grace`, disconnect the target so the probe is never held forever.
    pub fn schedule_release(self: &Arc<Self>, executor: ExecutorHandle) {
        let lease = Arc::clone(self);
        let grace = self.grace;
        tokio::spawn(async move {
            tokio::time::sleep(grace).await;
            if lease.active_clients() == 0 {
                let _ = executor
                    .call_async(
                        crate::op::OperationKind::Disconnect,
                        serde_json::json!({}),
                    )
                    .await;
            }
        });
    }
}
