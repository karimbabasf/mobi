// Pure render functions: each takes data and returns an HTML string, no side effects.
// Classes match the ported mockup CSS in styles.css.
import type { Agent, PaymentEvent, Roster, RosterEntry } from "./api";

export type Tab = "agents" | "payments";
export interface UiState {
  tab: Tab;
  selected: string | null;
}

const BRAND: Record<string, string> = {
  claude: "#D97757",
  codex: "#e6e6e6",
  gemini: "#4285F4",
  unknown: "#a78bfa",
};
const brand = (tool: string): string => BRAND[tool] ?? BRAND.unknown;
const money = (n: number): string => n.toFixed(2);

function esc(s: string): string {
  return s.replace(
    /[&<>"']/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!,
  );
}

function elapsed(iso: string): string {
  const ms = Date.now() - new Date(iso).getTime();
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h`;
  return `${Math.floor(h / 24)}d`;
}

function emptyState(msg: string): string {
  return `<div class="empty">${esc(msg)}</div>`;
}

export function renderHeader(r: Roster): string {
  const pend =
    r.pendingCount > 0 ? `<span class="gpend">${r.pendingCount} pending</span>` : "";
  return `<div class="glance">
    <span class="gb"><span class="live-dot"></span>${r.liveCount} live</span>
    <span class="gb gspend">◈ ${money(r.spendToday)} today</span>
    ${pend}
  </div>`;
}

export function renderTabs(tab: Tab, pendingCount: number): string {
  const badge = pendingCount > 0 ? `<i class="badge">${pendingCount}</i>` : "";
  return `<div class="seg">
    <span class="${tab === "agents" ? "on" : ""}" data-tab="agents">Agents</span>
    <span class="${tab === "payments" ? "on" : ""}" data-tab="payments">Payments${badge}</span>
  </div>`;
}

export function renderAgentsTab(r: Roster): string {
  if (r.entries.length === 0) return emptyState("No agents running.");
  return r.entries
    .map((e) => {
      const a = e.agent;
      const live = a.status !== "idle" ? "live" : "";
      const mark = e.pending
        ? ` <span class="tool" style="color:var(--yellow)">approval waiting</span>`
        : "";
      return `<div class="arow" data-agent="${esc(a.id)}">
      <div class="dot ${live}" style="background:${brand(a.tool)}"></div>
      <div class="abody">
        <div class="l1">${esc(a.name)} <span class="tool">${esc(a.terminal)}</span>${mark}</div>
        <div class="l2">${esc(a.currentAction)}</div>
      </div>
      <div class="time">${elapsed(a.startedAt)}</div>
    </div>`;
    })
    .join("");
}

export function renderPaymentsTab(r: Roster): string {
  if (r.entries.length === 0) return emptyState("No agents running.");
  return r.entries
    .map((e) => {
      const a = e.agent;
      const count = e.payments.filter(
        (p) => p.status === "settled" || p.status === "allowed",
      ).length;
      const right = e.pending
        ? `<span class="pendpill">◈ ${money(e.pending.amount)}</span>`
        : `<span class="amt">◈ ${money(e.spendToday)}</span>`;
      const sub = e.pending
        ? `wants to pay ${esc(e.pending.service)}`
        : `${count} payment${count === 1 ? "" : "s"} today`;
      return `<div class="arow" data-agent="${esc(a.id)}">
      <div class="dot" style="background:${brand(a.tool)}"></div>
      <div class="abody">
        <div class="l1">${esc(a.name)}</div>
        <div class="l2">${sub}</div>
      </div>
      <div class="time">${right}</div>
    </div>`;
    })
    .join("");
}

function ledgerRow(a: Agent, p: PaymentEvent): string {
  return `<div class="pl">
    <div class="dot" style="background:${brand(a.tool)}"></div>
    <div><div class="svc">${esc(p.service)}</div><div class="sub">${esc(a.name)} · ${elapsed(p.requestedAt)} ago</div></div>
    <span class="amt">◈ ${money(p.amount)}</span>
  </div>`;
}

export function renderDetail(e: RosterEntry): string {
  const a = e.agent;
  const approve = e.pending
    ? `<div class="approve">
        <div class="q">${esc(a.name)} wants to pay <b>◈ ${money(e.pending.amount)} USDC</b> to ${esc(e.pending.service)}</div>
        <div class="who">x402 · ${esc(e.pending.network)} · paused ${elapsed(e.pending.requestedAt)}</div>
        <div class="btns">
          <button class="deny" data-deny="${esc(e.pending.id)}">Deny</button>
          <button class="allow" data-allow="${esc(e.pending.id)}">Allow</button>
        </div>
      </div>`
    : "";
  const settled = e.payments.filter((p) => p.status === "settled" || p.status === "allowed");
  const denied = e.payments.filter((p) => p.status === "denied");
  const ledger =
    settled.length > 0
      ? [...settled].reverse().map((p) => ledgerRow(a, p)).join("")
      : `<div class="empty" style="min-height:auto;padding:10px">No settled payments yet.</div>`;
  return `<div class="detail">
    <div class="back" data-back="1">‹ Back</div>
    <div class="dhead">
      <span class="dot" style="background:${brand(a.tool)}"></span>
      <span class="dname">${esc(a.name)}</span>
      <span class="tool">${esc(a.terminal)}</span>
    </div>
    <div class="l2" style="margin:0 6px 8px">${esc(a.currentAction)}</div>
    ${approve}
    <div class="mini">
      <span><span class="lab">Spent today</span><br><span class="big"><span class="u">◈</span>${money(e.spendToday)}</span></span>
      <span class="lab">${settled.length} payment${settled.length === 1 ? "" : "s"}</span>
    </div>
    <div class="ledlab">Recent</div>
    ${ledger}
    ${denied.length > 0 ? `<div class="ledlab">Denied</div>${denied.map((p) => ledgerRow(a, p)).join("")}` : ""}
  </div>`;
}

export function renderPanel(state: UiState, r: Roster): string {
  if (state.selected) {
    const e = r.entries.find((x) => x.agent.id === state.selected);
    if (e) return renderHeader(r) + renderDetail(e);
  }
  const list = state.tab === "agents" ? renderAgentsTab(r) : renderPaymentsTab(r);
  return renderHeader(r) + renderTabs(state.tab, r.pendingCount) + `<div class="list">${list}</div>`;
}
