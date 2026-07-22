use crate::gate::{Decision, Gate};
use crate::model::*;
use crate::state::Store;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// The paid services a mocked agent hits, matching the kind of x402 endpoints agents use.
pub fn mock_service_pool() -> &'static [&'static str] {
    &[
        "replicate.com",
        "firecrawl.dev",
        "exa.ai",
        "api.tavily.com",
        "serper.dev",
    ]
}

/// Builds a fresh pending USDC-on-Base payment. Pure, so it is unit-testable.
pub fn synth_payment(id: &str, agent_id: Option<String>, service: &str, amount: f64) -> PaymentEvent {
    PaymentEvent {
        id: id.into(),
        agent_id,
        service: service.into(),
        amount,
        asset: "USDC".into(),
        network: "Base".into(),
        status: PaymentStatus::Pending,
        requested_at: now_rfc3339(),
        settled_at: None,
        tx_hash: None,
        source: "mock".into(),
    }
}

/// Behind the same interface a real x402 shim will use: emit a pending payment for a
/// live agent, route it through the Gate (which blocks the agent until the user decides),
/// then settle or drop it. The mock stream doubles as the test harness for the whole
/// money UI, per the spec.
pub async fn run_mock_sensor(store: Arc<Mutex<Store>>, gate: Arc<Gate>, app: AppHandle) {
    let mut n: u64 = 0;
    loop {
        tokio::time::sleep(Duration::from_secs(6)).await;
        let agent_id = { store.lock().unwrap().agent_ids().into_iter().next() };
        let Some(agent_id) = agent_id else { continue };
        n += 1;
        let id = format!("mock-{n}");
        let pool = mock_service_pool();
        let service = pool[(n as usize) % pool.len()];
        let amount = [0.02, 0.05, 0.12, 0.20][(n as usize) % 4];
        let pay = synth_payment(&id, Some(agent_id.clone()), service, amount);
        store.lock().unwrap().upsert_payment(pay);
        let _ = app.emit("roster-updated", ());

        let (tx, rx) = tokio::sync::oneshot::channel();
        gate.register(id.clone(), tx, Instant::now());
        match rx.await {
            Ok(Decision::Allow) => {
                store.lock().unwrap().set_payment_status(
                    &id,
                    PaymentStatus::Settled,
                    Some(now_rfc3339()),
                    Some(format!("0xmock{n:056x}")),
                );
            }
            _ => {
                store
                    .lock()
                    .unwrap()
                    .set_payment_status(&id, PaymentStatus::Denied, None, None);
            }
        }
        let _ = app.emit("roster-updated", ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synth_payment_is_pending_usdc_base_mock() {
        let p = synth_payment("p1", Some("a1".into()), "exa.ai", 0.02);
        assert_eq!(p.status, PaymentStatus::Pending);
        assert_eq!(p.asset, "USDC");
        assert_eq!(p.network, "Base");
        assert_eq!(p.source, "mock");
        assert_eq!(p.agent_id.as_deref(), Some("a1"));
        assert_eq!(p.service, "exa.ai");
    }
}
