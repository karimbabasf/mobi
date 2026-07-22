use crate::model::*;
use std::collections::HashMap;

/// In-memory store: the agents currently seen plus every payment seen this session.
/// One `roster()` call joins them into the view the UI reads. No persistence in v1,
/// so daily and session spend are the same number (the two fields let v2 add a daily
/// rollover without a schema change).
#[derive(Default)]
pub struct Store {
    agents: HashMap<String, Agent>,
    payments: Vec<PaymentEvent>, // insertion order preserved; deduped by id on upsert
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert_agent(&mut self, a: Agent) {
        self.agents.insert(a.id.clone(), a);
    }

    pub fn remove_agent(&mut self, id: &str) {
        self.agents.remove(id);
    }

    pub fn agent_ids(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }

    pub fn upsert_payment(&mut self, p: PaymentEvent) {
        if let Some(slot) = self.payments.iter_mut().find(|x| x.id == p.id) {
            *slot = p;
        } else {
            self.payments.push(p);
        }
    }

    /// Returns true if a payment with `id` existed and was updated.
    pub fn set_payment_status(
        &mut self,
        id: &str,
        status: PaymentStatus,
        settled_at: Option<String>,
        tx_hash: Option<String>,
    ) -> bool {
        if let Some(p) = self.payments.iter_mut().find(|x| x.id == id) {
            p.status = status;
            if settled_at.is_some() {
                p.settled_at = settled_at;
            }
            if tx_hash.is_some() {
                p.tx_hash = tx_hash;
            }
            true
        } else {
            false
        }
    }

    pub fn roster(&self) -> Roster {
        let mut entries: Vec<RosterEntry> = self
            .agents
            .values()
            .map(|a| {
                let mut agent = a.clone();
                let payments: Vec<PaymentEvent> = self
                    .payments
                    .iter()
                    .filter(|p| p.agent_id.as_deref() == Some(a.id.as_str()))
                    .cloned()
                    .collect();
                let pending = payments
                    .iter()
                    .find(|p| p.status == PaymentStatus::Pending)
                    .cloned();
                // A pending approval means the agent is blocked waiting on the user.
                if pending.is_some() {
                    agent.status = AgentStatus::WaitingApproval;
                }
                let spend_session: f64 = payments
                    .iter()
                    .filter(|p| {
                        matches!(p.status, PaymentStatus::Settled | PaymentStatus::Allowed)
                    })
                    .map(|p| p.amount)
                    .sum();
                RosterEntry {
                    agent,
                    spend_today: spend_session,
                    spend_session,
                    pending,
                    payments,
                }
            })
            .collect();
        entries.sort_by(|a, b| a.agent.name.cmp(&b.agent.name));
        let live_count = entries
            .iter()
            .filter(|e| e.agent.status != AgentStatus::Idle)
            .count();
        let pending_count = entries.iter().filter(|e| e.pending.is_some()).count();
        let spend_today = entries.iter().map(|e| e.spend_today).sum();
        Roster {
            entries,
            live_count,
            spend_today,
            pending_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(id: &str) -> Agent {
        Agent {
            id: id.into(),
            name: "Claude Code".into(),
            tool: "claude".into(),
            terminal: "iTerm".into(),
            status: AgentStatus::Working,
            current_action: "editing".into(),
            started_at: "2026-07-22T10:00:00Z".into(),
            wallet_addresses: vec![],
        }
    }
    fn pay(id: &str, agent: &str, amt: f64, status: PaymentStatus) -> PaymentEvent {
        PaymentEvent {
            id: id.into(),
            agent_id: Some(agent.into()),
            service: "replicate.com".into(),
            amount: amt,
            asset: "USDC".into(),
            network: "Base".into(),
            status,
            requested_at: "2026-07-22T10:01:00Z".into(),
            settled_at: None,
            tx_hash: None,
            source: "mock".into(),
        }
    }

    #[test]
    fn roster_joins_agents_payments_and_totals() {
        let mut s = Store::new();
        s.upsert_agent(agent("a1"));
        s.upsert_payment(pay("p1", "a1", 0.20, PaymentStatus::Settled));
        s.upsert_payment(pay("p2", "a1", 0.05, PaymentStatus::Pending));
        let r = s.roster();
        assert_eq!(r.entries.len(), 1);
        assert_eq!(r.live_count, 1);
        assert_eq!(r.pending_count, 1);
        let e = &r.entries[0];
        assert_eq!(e.payments.len(), 2);
        assert!((e.spend_session - 0.20).abs() < 1e-9); // only settled/allowed count as spent
        assert!(e.pending.is_some());
        assert_eq!(e.pending.as_ref().unwrap().id, "p2");
    }

    #[test]
    fn upsert_payment_dedups_by_id() {
        let mut s = Store::new();
        s.upsert_agent(agent("a1"));
        s.upsert_payment(pay("p1", "a1", 0.20, PaymentStatus::Pending));
        s.upsert_payment(pay("p1", "a1", 0.20, PaymentStatus::Settled));
        let r = s.roster();
        assert_eq!(r.entries[0].payments.len(), 1);
        assert_eq!(r.pending_count, 0);
    }

    #[test]
    fn set_payment_status_transitions() {
        let mut s = Store::new();
        s.upsert_agent(agent("a1"));
        s.upsert_payment(pay("p1", "a1", 0.20, PaymentStatus::Pending));
        assert!(s.set_payment_status("p1", PaymentStatus::Denied, None, None));
        assert!(!s.set_payment_status("missing", PaymentStatus::Denied, None, None));
        assert_eq!(s.roster().pending_count, 0);
    }

    #[test]
    fn waiting_agent_status_from_pending() {
        let mut s = Store::new();
        s.upsert_agent(agent("a1"));
        s.upsert_payment(pay("p1", "a1", 0.20, PaymentStatus::Pending));
        assert_eq!(s.roster().entries[0].agent.status, AgentStatus::WaitingApproval);
    }
}
