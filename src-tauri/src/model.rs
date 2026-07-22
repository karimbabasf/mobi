use serde::{Deserialize, Serialize};

/// Live work state of an agent, as shown in the roster.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentStatus {
    Working,
    Idle,
    WaitingApproval,
}

/// One AI agent running on the machine.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub id: String,
    pub name: String,
    /// Brand key used by the UI for the colour dot: "claude" | "codex" | "gemini" | "unknown".
    pub tool: String,
    pub terminal: String,
    pub status: AgentStatus,
    pub current_action: String,
    pub started_at: String, // RFC3339
    /// Registered wallet addresses only. Mobi never holds a private key.
    pub wallet_addresses: Vec<String>,
}

/// Lifecycle of a single x402 payment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PaymentStatus {
    Pending,
    Allowed,
    Denied,
    Settled,
    Failed,
}

/// One payment an agent makes (or tries to make). Mocked in v1.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentEvent {
    pub id: String,
    pub agent_id: Option<String>,
    pub service: String,
    pub amount: f64, // USDC
    pub asset: String,
    pub network: String,
    pub status: PaymentStatus,
    pub requested_at: String,
    pub settled_at: Option<String>,
    pub tx_hash: Option<String>,
    /// "shim" | "chain" | "mock".
    pub source: String,
}

/// One agent joined with its payments and rolled-up spend. The UI reads these.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterEntry {
    pub agent: Agent,
    pub payments: Vec<PaymentEvent>,
    pub spend_today: f64,
    pub spend_session: f64,
    pub pending: Option<PaymentEvent>,
}

/// The whole picture the dropdown renders: entries plus header totals.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Roster {
    pub entries: Vec<RosterEntry>,
    pub live_count: usize,
    pub spend_today: f64,
    pub pending_count: usize,
}
