import { allowPayment, denyPayment, getRoster, onRosterUpdated, syncTray } from "./api";
import { renderPanel, type UiState } from "./render";

const state: UiState = { tab: "agents", selected: null };
const app = document.getElementById("app")!;

async function refresh(): Promise<void> {
  const r = await getRoster();
  // Drop a stale selection if that agent went away.
  if (state.selected && !r.entries.some((e) => e.agent.id === state.selected)) {
    state.selected = null;
  }
  app.innerHTML = renderPanel(state, r);
  void syncTray(r.pendingCount > 0);
}

app.addEventListener("click", async (ev) => {
  const t = ev.target as HTMLElement;

  const tab = t.closest("[data-tab]");
  if (tab) {
    state.tab = tab.getAttribute("data-tab") as UiState["tab"];
    state.selected = null;
    await refresh();
    return;
  }

  const allow = t.closest("[data-allow]");
  if (allow) {
    await allowPayment(allow.getAttribute("data-allow")!);
    return;
  }

  const deny = t.closest("[data-deny]");
  if (deny) {
    await denyPayment(deny.getAttribute("data-deny")!);
    return;
  }

  if (t.closest("[data-back]")) {
    state.selected = null;
    await refresh();
    return;
  }

  const row = t.closest("[data-agent]");
  if (row) {
    state.selected = row.getAttribute("data-agent");
    await refresh();
  }
});

void onRosterUpdated(() => {
  void refresh();
});

void refresh();

// Keep elapsed times and the demo stream feeling live.
setInterval(() => void refresh(), 2000);
