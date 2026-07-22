# Mobi: Design Spec

Date: 2026-07-22
Status: Direction approved, pending spec review
Owner: Karim

## What Mobi is

Mobi is a native macOS menu-bar app: one glance at what the AI agents on your Mac are doing and what they are spending. It is the local, cross-agent buyer-side control plane for x402 payments. Every agent on your machine can pay through Mobi, so Mobi sees every payment, can stop any payment before it settles, and unifies work and money into one dropdown that hangs off your toolbar.

The reference for the "work" half is Vibe Island (a notch app that monitors many agent CLIs). Mobi matches that monitoring and adds the half nobody has built: the money.

## Why the buyer layer

x402 is not an app. It is plumbing: an HTTP 402 status plus an EIP-3009 stablecoin settlement standard, embedded into other software. It shows up at four layers:

- Facilitator (who settles onchain): Coinbase, Cloudflare, AWS, Stripe, Visa. Owned by giants. Not a product a solo builder ships.
- Seller (who charges per call): API and agent-output monetization. A crowded dev-tool market, unrelated to the agents on your machine.
- Discovery (how agents find paid services): Coinbase's Bazaar directory. A marketplace with network effects.
- Buyer/client (the agent that pays): in practice an SDK inside an agent, plus a wallet, plus x402 middleware. There is no human-facing client brand here.

Mobi targets the buyer layer. The tell is that no consumer-facing x402 client exists: everyone shipped an SDK, nobody shipped the control plane a person uses to watch and govern what their agents buy. That seat is open.

It holds up against a single framework absorbing it because any one framework (Cursor, AgentKit, a given MCP host) sees only its own spend. A person runs several agents at once. The cross-agent, cross-rail, local, buy-once cockpit is exactly what no single framework can be, the same reason Vibe Island exists to unify many agent CLIs.

## How Mobi sees and controls money

A single x402 payment is: hit a 402, sign an EIP-3009 transferWithAuthorization, retry with the payment header, a facilitator verifies then settles USDC on Base. Mobi uses two mechanisms against that flow:

1. In-process shim (the gate). A small package the user adds to an agent or MCP server, wrapping the x402 client (x402-fetch, x402-axios, the Python client, or an MCP tool call). It intercepts the moment right before the payment is signed and asks Mobi to allow or deny over localhost IPC. This is the only clean pre-settlement gate: in-process, no TLS interception, no macOS entitlements. It is also where real agent payments happen today, since MCP is the dominant integration (Coinbase Payments MCP lets Claude pay with no API keys, and an MCP server runs in a shimmable process). No private key leaves the agent; Mobi holds no custody.

2. On-chain watcher (the backstop). Mobi subscribes to USDC Transfer events on Base for the wallet addresses the user registers per agent. It needs no cooperation from any agent and gives a global view, but it is observe-only and post-hoc: a settlement is a plain EIP-3009 transfer, so the chain alone cannot prove a transfer was x402 or which resource it bought. App-layer context comes from the shim; the chain confirms settlement and catches anything the shim did not see.

A forward proxy could gate all traffic but is deferred: Node and axios ignore proxy env vars by default, GUI-launched agents miss shell env, and it requires dual CA trust. Too many silent failures for v1.

Constraint that shapes the UX: a gated payment blocks its agent, because the shim pauses before signing. A pending approval is a stalled agent, so clearing approvals must be fast and no agent can hang forever (wait-timer plus a timeout action).

## Goals and non-goals

Goals:
- One glance at every agent's work and spend, from the menu bar.
- Allow or stop any payment on the path Mobi is wired into, before it settles.
- No custody: agents keep their own keys.
- Cross-agent and cross-rail by design.

Non-goals (v1):
- No policy engine (per-agent caps, allowlists, auto-approval). Approvals are manual.
- No custody or wallet-holding.
- No facilitator, seller, or discovery product.
- No forward-proxy interception.

## Architecture

Six units, each with one responsibility, a defined interface, and independent tests.

1. Agent Monitor
- Does: detect running agent CLIs and their live state (name, tool, terminal, current action, status, start time).
- Interface: emits Agent records to the State store.
- Depends on: per-agent state sources (session files, hooks, process inspection). v1 covers Claude Code and Codex.

2. Payment Sensor
- Does: collect payment events from two sources and normalize them.
- Sources: (a) shim ingress over localhost IPC (a local HTTP or WebSocket or unix-socket server Mobi runs), (b) the on-chain watcher (Base RPC or WebSocket subscription for registered addresses).
- Interface: emits normalized PaymentEvent records; forwards pre-sign events to the Gate.

3. Gate
- Does: hold a pre-sign payment, surface it as pending, apply the user's Allow or Deny, reply to the shim to sign or abort.
- v1: manual per payment, no rules. Includes a wait-timer and a timeout action (default: hold, configurable to deny).

4. Wallet registry
- Does: map an agent to one or more wallet addresses, so the watcher knows what to watch and spend attributes to the right agent.
- Holds addresses only, never keys.

5. State store
- Does: merge work and money into a single agent roster the UI reads; correlate a shim event with its on-chain settlement so a payment is counted once; keep per-agent and daily totals and history.
- Interface: reactive reads for the UI; write APIs for Monitor, Sensor, Gate.

6. Menu-bar UI
- Does: render the dropdown. Icon glance states, always-on header, Agents and Payments tabs over the same roster, and a per-agent detail panel on click.

## Data flow

- Agent Monitor to State (work state).
- Shim to IPC to Payment Sensor to Gate (if the event is pre-sign) to State. The Gate's decision returns to the shim.
- On-chain watcher to Payment Sensor to State (settlements, cross-check).
- UI reads State reactively. Allow or Deny goes to the Gate. Jump-to-terminal goes to the Monitor.

## Data model (v1)

Agent: id, name, tool, terminal, status (working, idle, waiting-approval), currentAction, startedAt, walletAddresses.

PaymentEvent: id, agentId (nullable if only seen on-chain), service, amount, asset (USDC), network (Base), status (pending, allowed, denied, settled, failed), requestedAt, settledAt, txHash (nullable), source (shim or chain).

Roster: the joined view the UI reads. Per agent: its work state, its payments, rolled-up spend (today, session), and any pending approval.

## Approvals and concurrency

The model is agent-centric, which answers "what if several agents pay at once":

- Both tabs show the same agent roster. The Agents tab shows each agent's work; the Payments tab shows each agent's spend and a pending marker.
- Clicking an agent opens its full detail panel: current work, spend, payment history, and any pending approval with Allow and Deny.
- Several agents paying at once means several agents in the list carry a pending marker. The user clicks into whichever needs them and decides. No rules engine.
- Each pending payment shows a wait-timer; a per-agent timeout action prevents an agent from hanging forever.

## UI spec

Menu-bar icon (the always-on glance, nothing open):
- Idle: the mark only.
- Working: activity pulse.
- Spending: optional live spend readout.
- Approval waiting: amber dot.
- Default: activity pulse plus amber-on-approval. The final resting behavior is an open choice (minimal dot, live spend, or pulse).

Dropdown:
- Always-on header across both tabs: live agent count, spend today, pending count.
- Segmented control: Agents and Payments, over the same roster. Payments carries a count badge when approvals wait.
- Agents tab: rows of agent, tool, terminal, current action, time. Click a row to jump to its terminal.
- Payments tab: rows of agent with spend and a pending marker. Click a row to open the agent detail.
- Agent detail: back to the roster, the agent's current work, any pending approval (Allow or Deny), spend today, and the settled payment history.

## Stack

Tauri (Rust backend, web frontend).
- Frontend: the HTML and CSS dropdown from the brainstorm mockups ports in directly.
- Backend (Rust): the localhost IPC server for shim ingress, the Base on-chain watcher (alloy or equivalent), the State store, and the tray plus dropdown-panel window.
- macOS: a tray icon plus a positioned webview panel under the toolbar. Panel positioning and vibrancy are the manual work here (the one thing SwiftUI would give for free).
- The shim is a separate JS, TS, and Python package regardless of the app stack.

## v1 scope

Real:
- Agent Monitor for Claude Code and Codex (live work state).
- The full menu-bar UI (icon states, header, tabs, agent detail).
- The State store.
- The complete Allow or Deny and agent-detail flow.

Mocked:
- The Payment Sensor emits synthetic pending and settled events behind the same interface the real shim will use. The whole money UI and approval flow is real and demoable without touching crypto.

Deferred to v2:
- The real shim (npm package plus a reference MCP-server wrapper).
- The real Base watcher and wallet-registry wiring.
- Multi-rail support beyond x402 on Base.
- Any policy or limits.

Because the v1 mock stream uses the same interface as the real shim, the mock doubles as the test harness.

## Testing

- Agent Monitor: unit-test the state parser against captured fixtures of each agent's session output.
- Payment Sensor, Gate, State store: drive synthetic event streams (the v1 mock) and assert roster contents, totals, and approval transitions (pending to allowed, pending to denied, timeout).
- UI: component states rendered and checked for idle, working, spending, one pending, several pending, and agent detail.

## Risks and open questions

- Absorption risk: agent frameworks keep swallowing wallets and buyers never feel they need a separate app. Counter: nobody runs one agent, and local, private, buy-once beats hosted lock-in.
- Protocol churn: x402 shipped a breaking v2 on 2025-12-09 (the payment header was renamed, the network string moved to CAIP-2). v1 and v2 run at once. Support both and re-verify field shapes at build time.
- Monitoring closed agents: Mobi can observe Claude Code and Codex state but cannot gate their internal payments directly. The gate rides the shim and MCP path, which is where real payments are. Observe covers the rest.
- Data integrity: separate x402 protocol volume from the "x402 token" memecoin, and filter gamed or wash-traded transactions, whenever real on-chain or aggregate numbers appear.
- Open UI choice: the icon's resting behavior.

## Milestones

v1: monitoring plus full UI plus mocked payments, demoable end to end. The thing Karim holds first.
v2: wire the real shim (SDK plus MCP wrapper) and the Base watcher behind the existing interfaces; register wallets; real allow or deny gating a real payment.
