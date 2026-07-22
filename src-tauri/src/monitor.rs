use crate::model::*;
use crate::sensor::now_rfc3339;
use crate::state::Store;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use sysinfo::System;

/// Maps a process/binary name to (tool_key, display_name). Only real agent CLIs match;
/// everything else (node, python, the shell) returns None.
pub fn classify_tool(name: &str) -> Option<(&'static str, &'static str)> {
    let n = name.to_ascii_lowercase();
    if n == "claude" || n.starts_with("claude-code") || n.starts_with("claude ") {
        return Some(("claude", "Claude Code"));
    }
    if n == "codex" || n.starts_with("codex") {
        return Some(("codex", "Codex"));
    }
    if n == "gemini" {
        return Some(("gemini", "Gemini"));
    }
    None
}

/// Synthetic agents so the dropdown is never empty on a first look (used when no real
/// agent CLI is running). Clearly a demo: fixed ids prefixed `demo-`.
pub fn demo_agents() -> Vec<Agent> {
    let now = now_rfc3339();
    vec![
        Agent {
            id: "demo-claude".into(),
            name: "Claude Code".into(),
            tool: "claude".into(),
            terminal: "iTerm, mobi".into(),
            status: AgentStatus::Working,
            current_action: "editing src-tauri/src/state.rs".into(),
            started_at: now.clone(),
            wallet_addresses: vec![],
        },
        Agent {
            id: "demo-codex".into(),
            name: "Codex".into(),
            tool: "codex".into(),
            terminal: "Terminal, api".into(),
            status: AgentStatus::Working,
            current_action: "running tests".into(),
            started_at: now,
            wallet_addresses: vec![],
        },
    ]
}

/// Scans running processes for known agent CLIs. Best-effort: name, pid, working dir.
pub fn scan_agents(sys: &System) -> Vec<Agent> {
    let mut out = Vec::new();
    for (pid, proc_) in sys.processes() {
        let name = proc_.name().to_string_lossy();
        if let Some((tool, display)) = classify_tool(&name) {
            let cwd = proc_
                .cwd()
                .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
                .unwrap_or_else(|| "terminal".into());
            out.push(Agent {
                id: format!("proc-{pid}"),
                name: display.into(),
                tool: tool.into(),
                terminal: cwd,
                status: AgentStatus::Working,
                current_action: "active".into(),
                started_at: now_rfc3339(),
                wallet_addresses: vec![],
            });
        }
    }
    out
}

/// Refreshes the store's agent set from a live process scan, falling back to demo agents
/// when nothing real is running. Agents that vanished are removed.
pub fn refresh_agents(store: &Arc<Mutex<Store>>, sys: &mut System) {
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let mut agents = scan_agents(sys);
    if agents.is_empty() {
        agents = demo_agents();
    }
    let live: HashSet<String> = agents.iter().map(|a| a.id.clone()).collect();
    let mut s = store.lock().unwrap();
    for id in s.agent_ids() {
        if !live.contains(&id) {
            s.remove_agent(&id);
        }
    }
    for a in agents {
        s.upsert_agent(a);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_known_agents() {
        assert_eq!(classify_tool("claude").map(|t| t.0), Some("claude"));
        assert_eq!(classify_tool("claude-code").map(|t| t.0), Some("claude"));
        assert_eq!(classify_tool("codex").map(|t| t.0), Some("codex"));
        assert_eq!(classify_tool("node"), None);
    }

    #[test]
    fn demo_agents_are_populated() {
        let a = demo_agents();
        assert!(a.len() >= 2);
        assert!(a.iter().any(|x| x.tool == "claude"));
        assert!(a.iter().all(|x| x.id.starts_with("demo-")));
    }
}
