use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

/// The user's answer to a held payment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
}

/// Holds payments that are paused pre-signing, waiting on the user. A producer
/// (the mock sensor in v1, a real shim in v2) registers a payment and awaits the
/// decision; the UI resolves it. This is the one clean pre-settlement gate.
#[derive(Default)]
pub struct Gate {
    pending: Mutex<HashMap<String, (Instant, oneshot::Sender<Decision>)>>,
}

impl Gate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, payment_id: String, tx: oneshot::Sender<Decision>, requested_at: Instant) {
        self.pending
            .lock()
            .unwrap()
            .insert(payment_id, (requested_at, tx));
    }

    /// Delivers a decision to the waiting producer. Returns false if nothing was pending under that id.
    pub fn resolve(&self, payment_id: &str, d: Decision) -> bool {
        if let Some((_, tx)) = self.pending.lock().unwrap().remove(payment_id) {
            let _ = tx.send(d); // receiver may have dropped; ignore
            true
        } else {
            false
        }
    }

    // Query surface exercised by tests and reserved for the v2 configurable timeout
    // action. v1's wait-timer is frontend-driven off `requestedAt`, so these are not
    // yet called from the running app.
    #[allow(dead_code)]
    pub fn pending_ids(&self) -> Vec<String> {
        self.pending.lock().unwrap().keys().cloned().collect()
    }

    /// Reports ids held longer than `ttl`. v1 timeout action is "hold": overdue ids
    /// are surfaced (badge) but not auto-denied, so no agent is decided against silently.
    #[allow(dead_code)]
    pub fn timeout_sweep(&self, now: Instant, ttl: Duration) -> Vec<String> {
        self.pending
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, (at, _))| now.duration_since(*at) >= ttl)
            .map(|(id, _)| id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resolve_delivers_decision_and_clears() {
        let g = Gate::new();
        let (tx, rx) = oneshot::channel();
        g.register("p1".into(), tx, Instant::now());
        assert_eq!(g.pending_ids(), vec!["p1".to_string()]);
        assert!(g.resolve("p1", Decision::Allow));
        assert_eq!(rx.await.unwrap(), Decision::Allow);
        assert!(g.pending_ids().is_empty());
        assert!(!g.resolve("p1", Decision::Allow)); // already resolved
    }

    #[test]
    fn timeout_sweep_returns_overdue() {
        let g = Gate::new();
        let (tx, _rx) = oneshot::channel();
        let old = Instant::now() - Duration::from_secs(120);
        g.register("p1".into(), tx, old);
        let overdue = g.timeout_sweep(Instant::now(), Duration::from_secs(60));
        assert_eq!(overdue, vec!["p1".to_string()]);
    }
}
