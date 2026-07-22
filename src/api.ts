// Typed mirror of the Rust model (serde camelCase) plus the command/event wrappers.
// When not running inside Tauri (a plain browser, for UI verification), a small in-memory
// shim serves a fixture roster and simulates allow/deny so the whole UI is exercisable.

export type AgentStatus = "working" | "idle" | "waitingApproval";
export type PaymentStatus = "pending" | "allowed" | "denied" | "settled" | "failed";

export interface Agent {
  id: string;
  name: string;
  tool: string;
  terminal: string;
  status: AgentStatus;
  currentAction: string;
  startedAt: string;
  walletAddresses: string[];
}

export interface PaymentEvent {
  id: string;
  agentId: string | null;
  service: string;
  amount: number;
  asset: string;
  network: string;
  status: PaymentStatus;
  requestedAt: string;
  settledAt: string | null;
  txHash: string | null;
  source: string;
}

export interface RosterEntry {
  agent: Agent;
  payments: PaymentEvent[];
  spendToday: number;
  spendSession: number;
  pending: PaymentEvent | null;
}

export interface Roster {
  entries: RosterEntry[];
  liveCount: number;
  spendToday: number;
  pendingCount: number;
}

const IS_TAURI = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// --- Real Tauri path -------------------------------------------------------

async function tauriGetRoster(): Promise<Roster> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<Roster>("get_roster");
}
async function tauriAllow(id: string): Promise<boolean> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<boolean>("allow_payment", { id });
}
async function tauriDeny(id: string): Promise<boolean> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<boolean>("deny_payment", { id });
}
async function tauriSyncTray(pending: boolean): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("sync_tray", { pending });
}
async function tauriOnRosterUpdated(cb: () => void): Promise<() => void> {
  const { listen } = await import("@tauri-apps/api/event");
  return listen("roster-updated", () => cb());
}

// --- Browser dev shim ------------------------------------------------------

function minutesAgo(n: number): string {
  return new Date(Date.now() - n * 60_000).toISOString();
}
function fixturePayment(
  id: string,
  agentId: string,
  service: string,
  amount: number,
  status: PaymentStatus,
  requestedAt: string,
): PaymentEvent {
  return {
    id,
    agentId,
    service,
    amount,
    asset: "USDC",
    network: "Base",
    status,
    requestedAt,
    settledAt: status === "settled" ? requestedAt : null,
    txHash: status === "settled" ? "0xmock000000000000000000000000000000000000000000000000000000000001" : null,
    source: "mock",
  };
}

const shimEntries: RosterEntry[] = [
  {
    agent: {
      id: "demo-claude",
      name: "Claude Code",
      tool: "claude",
      terminal: "iTerm, mobi",
      status: "working",
      currentAction: "Writing middleware.ts",
      startedAt: minutesAgo(27),
      walletAddresses: [],
    },
    payments: [
      fixturePayment("s1", "demo-claude", "firecrawl.dev", 0.05, "settled", minutesAgo(31)),
      fixturePayment("p1", "demo-claude", "replicate.com", 0.12, "pending", minutesAgo(0)),
    ],
    spendToday: 0,
    spendSession: 0,
    pending: null,
  },
  {
    agent: {
      id: "demo-codex",
      name: "Codex",
      tool: "codex",
      terminal: "Terminal, api",
      status: "working",
      currentAction: "npm test: 3 passed",
      startedAt: minutesAgo(63),
      walletAddresses: [],
    },
    payments: [
      fixturePayment("s2", "demo-codex", "exa.ai", 0.02, "settled", minutesAgo(12)),
      fixturePayment("s3", "demo-codex", "api.tavily.com", 1.08, "settled", minutesAgo(44)),
    ],
    spendToday: 0,
    spendSession: 0,
    pending: null,
  },
];

const shimSubs: Array<() => void> = [];

function shimCompute(): Roster {
  for (const e of shimEntries) {
    const pend = e.payments.find((p) => p.status === "pending") ?? null;
    e.pending = pend;
    if (pend) {
      e.agent.status = "waitingApproval";
    } else if (e.agent.status === "waitingApproval") {
      e.agent.status = "working";
    }
    e.spendSession = e.payments
      .filter((p) => p.status === "settled" || p.status === "allowed")
      .reduce((s, p) => s + p.amount, 0);
    e.spendToday = e.spendSession;
  }
  shimEntries.sort((a, b) => a.agent.name.localeCompare(b.agent.name));
  return {
    entries: shimEntries,
    liveCount: shimEntries.filter((e) => e.agent.status !== "idle").length,
    spendToday: shimEntries.reduce((s, e) => s + e.spendToday, 0),
    pendingCount: shimEntries.filter((e) => e.pending).length,
  };
}

function shimResolve(id: string, status: PaymentStatus): boolean {
  for (const e of shimEntries) {
    const p = e.payments.find((x) => x.id === id);
    if (p) {
      p.status = status;
      if (status === "settled") {
        p.settledAt = new Date().toISOString();
        p.txHash = "0xmock000000000000000000000000000000000000000000000000000000000002";
      }
      shimSubs.forEach((cb) => cb());
      return true;
    }
  }
  return false;
}

// --- Public surface --------------------------------------------------------

export const getRoster = (): Promise<Roster> =>
  IS_TAURI ? tauriGetRoster() : Promise.resolve(shimCompute());

export const allowPayment = (id: string): Promise<boolean> =>
  IS_TAURI ? tauriAllow(id) : Promise.resolve(shimResolve(id, "settled"));

export const denyPayment = (id: string): Promise<boolean> =>
  IS_TAURI ? tauriDeny(id) : Promise.resolve(shimResolve(id, "denied"));

export const syncTray = (pending: boolean): Promise<void> =>
  IS_TAURI ? tauriSyncTray(pending) : Promise.resolve();

export const onRosterUpdated = (cb: () => void): Promise<() => void> => {
  if (IS_TAURI) return tauriOnRosterUpdated(cb);
  shimSubs.push(cb);
  return Promise.resolve(() => {
    const i = shimSubs.indexOf(cb);
    if (i >= 0) shimSubs.splice(i, 1);
  });
};
