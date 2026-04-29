//! Unit tests for `assemble_heartbeat` field derivation per D-08, D-09, D-10, D-11.
//! No disk I/O — `assemble_heartbeat` is pure (D-05). The handle_event smoke
//! tests own the round-trip-via-disk side (must_have-1).

use holt_hooks::{Env, HookEvent, HookStdin, assemble_heartbeat};

fn fixture_v2119_pretooluse() -> HookStdin {
    let bytes = std::fs::read("tests/fixtures/cc-stdin/v2.1.119/PreToolUse.json")
        .expect("v2.1.119/PreToolUse.json fixture must exist (Task 1)");
    holt_hooks::stdin::parse(&bytes).expect("v2.1.119 fixture must parse")
}

fn fixture_pre_2198() -> HookStdin {
    let bytes = std::fs::read("tests/fixtures/cc-stdin/pre-2.1.98/PreToolUse.json")
        .expect("pre-2.1.98/PreToolUse.json fixture must exist (Task 1)");
    holt_hooks::stdin::parse(&bytes).expect("pre-2.1.98 fixture must parse")
}

fn env() -> Env {
    Env {
        writer_version: "test-0.0.0",
        pid: 12345,
        now_iso: "2026-04-28T10:00:00Z".to_string(),
    }
}

#[test]
fn cwd_label_uses_workspace_git_worktree_when_present() {
    let stdin = fixture_v2119_pretooluse();
    let hb = assemble_heartbeat(HookEvent::PreToolUse, &stdin, &env());
    assert_eq!(
        hb.cwd_label, "myrepo/feature-branch",
        "must use workspace.git_worktree verbatim per D-08 tier 1"
    );
    assert!(!hb.cwd_label.is_empty(), "must_have-5: cwd_label non-empty");
}

#[test]
fn cwd_label_falls_back_to_basename_when_workspace_absent() {
    let stdin = fixture_pre_2198();
    let hb = assemble_heartbeat(HookEvent::PreToolUse, &stdin, &env());
    assert_eq!(
        hb.cwd_label, "myrepo",
        "must derive from basename(cwd) when workspace.git_worktree absent"
    );
    assert!(
        !hb.cwd_label.is_empty(),
        "must_have-5: cwd_label non-empty even without workspace"
    );
}

#[test]
fn current_tool_is_some_on_pretooluse() {
    let stdin = fixture_v2119_pretooluse();
    let hb = assemble_heartbeat(HookEvent::PreToolUse, &stdin, &env());
    assert!(
        hb.current_tool.is_some(),
        "D-09: PreToolUse → Some(tool_name)"
    );
}

#[test]
fn current_tool_is_none_on_post_stop_notification_sessionstart() {
    // tool_name present in fixture but irrelevant — D-09 forces None for non-Pre events.
    let stdin = fixture_v2119_pretooluse();
    for ev in [
        HookEvent::PostToolUse,
        HookEvent::Stop,
        HookEvent::Notification,
        HookEvent::SessionStart,
    ] {
        let hb = assemble_heartbeat(ev, &stdin, &env());
        assert!(hb.current_tool.is_none(), "D-09: {ev:?} must yield None");
    }
}

#[test]
fn blocked_on_always_none_at_v01() {
    let stdin = fixture_v2119_pretooluse();
    for ev in [
        HookEvent::PreToolUse,
        HookEvent::PostToolUse,
        HookEvent::Stop,
        HookEvent::Notification,
        HookEvent::SessionStart,
    ] {
        let hb = assemble_heartbeat(ev, &stdin, &env());
        assert!(
            hb.blocked_on.is_none(),
            "D-10: blocked_on must be None at v0.1"
        );
    }
}

#[test]
fn writer_version_plumbed_from_env() {
    let stdin = fixture_v2119_pretooluse();
    let hb = assemble_heartbeat(HookEvent::PreToolUse, &stdin, &env());
    assert_eq!(
        hb.writer_version, "test-0.0.0",
        "D-11: writer_version comes from Env, not the holt-hooks crate's CARGO_PKG_VERSION"
    );
}

#[test]
fn schema_version_is_1() {
    let stdin = fixture_v2119_pretooluse();
    let hb = assemble_heartbeat(HookEvent::PreToolUse, &stdin, &env());
    assert_eq!(
        hb.schema_version, 1,
        "HOOK-05: schema_version is exactly 1 at v0.1"
    );
}

#[test]
fn defensive_parse_succeeds_on_unknown_fields() {
    // The v2.1.119 PreToolUse fixture contains "effort" with "level": "xhigh" —
    // a field not declared on HookStdin. PITFALLS H5 mandates this parse cleanly.
    let stdin = fixture_v2119_pretooluse();
    assert!(
        !stdin.session_id.is_empty(),
        "D-04 defensive parse must surface session_id"
    );
}
