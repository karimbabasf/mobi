# Mobi

Menu-bar control plane for the AI agents on your Mac: one glance at what they are doing and what they are spending. Mobi is the local, cross-agent buyer-side view for x402 payments. It watches the agent CLIs running on your machine and, in v1, shows a mocked payment stream routed through the same approve/deny gate a real x402 shim will use.

v1 is real agent monitoring plus the full menu-bar UI, with payments mocked behind the real interface. The in-process x402 shim and the on-chain Base watcher are v2; they slot in behind the same interfaces without UI changes. Mobi never holds a private key.

## Stack

Tauri 2 (Rust backend, vanilla-TypeScript + Vite frontend). macOS only.

## Run

```
npm install
npm run tauri dev
```

A diamond icon appears in the menu bar. Click it to open the panel. With no real agent CLI running, Mobi shows demo agents so the UI is populated, and the mock sensor emits a payment every few seconds that you approve or deny.

## Build

```
npm run tauri build
```

The bundled app lands at `src-tauri/target/release/bundle/macos/Mobi.app`.

## Test

```
cargo test --manifest-path src-tauri/Cargo.toml   # Rust: store, gate, sensor, monitor
npm run build                                      # frontend typecheck + bundle
```

You can also exercise the whole UI in a plain browser with `npm run dev`: when Mobi is not running inside Tauri, a small in-memory shim serves a fixture roster and simulates allow/deny.

## Icons

The mark is `assets/icon.svg`. Regenerate the app and tray icons with:

```
npm run gen:icon
npm run tauri icon app-icon.png
```

## Layout

```
src-tauri/src/model.rs      data model (Agent, PaymentEvent, Roster)
src-tauri/src/state.rs      in-memory store and roster join
src-tauri/src/gate.rs       approve/deny gate for pending payments
src-tauri/src/sensor.rs     mock payment sensor (v2: real x402 shim ingress)
src-tauri/src/monitor.rs    agent process scan (Claude Code, Codex) with demo fallback
src-tauri/src/tray.rs       menu-bar tray icon and panel positioning
src-tauri/src/commands.rs   Tauri commands the UI calls
src/render.ts               pure render functions
src/api.ts                  typed model, command wrappers, browser dev shim
src/main.ts                 UI state and event wiring
```
