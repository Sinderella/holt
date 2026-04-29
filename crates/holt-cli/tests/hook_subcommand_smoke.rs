//! End-to-end integration tests for `holt hook <event>`.
//!
//! Runs the release binary against the v2.1.119 fixtures, asserts each
//! event's heartbeat is written to the canonical XDG path, parses cleanly
//! via `holt_schemas::read_heartbeat`, and exhibits the per-event
//! `current_tool` policy (must_have-2 end-to-end). Also exercises must_have-1
//! through the binary surface (file shape + 0o600 perms + read_heartbeat
//! round-trip).
//!
//! Tests are serialized via a global `Mutex` because they all spawn the
//! `holt` binary with `XDG_RUNTIME_DIR` pointed at a per-test tempdir; the
//! mutex guards against the rare case where two parallel tests share the
//! exact same fixture session_id (defence-in-depth — each test uses its own
//! tempdir, so collisions are impossible in practice).

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;

use holt_schemas::read_heartbeat;

static ENV_LOCK: Mutex<()> = Mutex::new(());

const HOLT_BIN: &str = env!("CARGO_BIN_EXE_holt");

fn fixture_path(name: &str) -> PathBuf {
    // Tests for holt-cli run from `crates/holt-cli`; fixtures live in
    // `crates/holt-hooks/tests/fixtures/cc-stdin/v2.1.119/<name>.json`.
    PathBuf::from("../holt-hooks/tests/fixtures/cc-stdin/v2.1.119").join(name)
}

fn fixture_session_id(path: &std::path::Path) -> String {
    let bytes = std::fs::read(path).expect("fixture readable");
    let v: serde_json::Value = serde_json::from_slice(&bytes).expect("fixture parses");
    v["session_id"]
        .as_str()
        .expect("fixture has session_id")
        .to_string()
}

fn run_holt_hook(
    event: &str,
    fixture: &std::path::Path,
    xdg: &std::path::Path,
) -> std::process::Output {
    let stdin_bytes = std::fs::read(fixture).expect("fixture readable");
    let mut child = Command::new(HOLT_BIN)
        .arg("hook")
        .arg(event)
        .env("XDG_RUNTIME_DIR", xdg)
        .env_remove("TMPDIR")
        .env_remove("XDG_CACHE_HOME")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn holt");
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(&stdin_bytes)
        .unwrap();
    drop(child.stdin.take());
    child.wait_with_output().expect("wait holt")
}

#[test]
fn must_have_1_hook_pre_tool_use_writes_canonical_xdg_path() {
    let _g = ENV_LOCK.lock().unwrap();
    let xdg = tempfile::tempdir().unwrap();
    let fixture = fixture_path("PreToolUse.json");

    let out = run_holt_hook("PreToolUse", &fixture, xdg.path());
    assert_eq!(
        out.status.code(),
        Some(0),
        "hook must exit 0 (stderr={})",
        String::from_utf8_lossy(&out.stderr)
    );

    let sid = fixture_session_id(&fixture);
    let target = xdg
        .path()
        .join("holt")
        .join("sessions")
        .join(format!("{sid}.json"));
    let hb = read_heartbeat(&target)
        .expect("read_heartbeat returns Result")
        .expect("heartbeat must exist");
    assert_eq!(hb.schema_version, 1);
    assert_eq!(hb.session_id, sid);
    assert!(
        !hb.writer_version.is_empty(),
        "HOOK-06: writer_version populated by binary"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "must_have-1: 0600 perms via the binary path");
    }
}

#[test]
fn must_have_2_all_five_events_round_trip_via_binary() {
    let _g = ENV_LOCK.lock().unwrap();
    let xdg = tempfile::tempdir().unwrap();

    for (event, fixture_name, expect_tool) in [
        ("PreToolUse", "PreToolUse.json", true),
        ("PostToolUse", "PostToolUse.json", false),
        ("Stop", "Stop.json", false),
        ("Notification", "Notification.json", false),
        ("SessionStart", "SessionStart.json", false),
    ] {
        let fixture = fixture_path(fixture_name);
        let out = run_holt_hook(event, &fixture, xdg.path());
        assert_eq!(
            out.status.code(),
            Some(0),
            "{event}: must exit 0 (stderr={})",
            String::from_utf8_lossy(&out.stderr)
        );

        let sid = fixture_session_id(&fixture);
        let target = xdg
            .path()
            .join("holt")
            .join("sessions")
            .join(format!("{sid}.json"));
        let hb = read_heartbeat(&target)
            .unwrap_or_else(|e| panic!("{event}: read_heartbeat err: {e:?}"))
            .unwrap_or_else(|| panic!("{event}: heartbeat missing at {target:?}"));

        if expect_tool {
            assert!(
                hb.current_tool.is_some(),
                "{event}: D-09 PreToolUse → Some(tool)"
            );
        } else {
            assert!(hb.current_tool.is_none(), "{event}: D-09 non-Pre → None");
        }

        // Every event populates writer_version (HOOK-06) and uses
        // schema_version=1 (D-05).
        assert_eq!(hb.schema_version, 1, "{event}: schema_version=1");
        assert!(
            !hb.writer_version.is_empty(),
            "{event}: writer_version populated"
        );
    }
}

#[test]
fn must_have_2_garbage_stdin_exits_zero_no_panic() {
    let _g = ENV_LOCK.lock().unwrap();
    let xdg = tempfile::tempdir().unwrap();

    let mut child = Command::new(HOLT_BIN)
        .arg("hook")
        .arg("PreToolUse")
        .env("XDG_RUNTIME_DIR", xdg.path())
        .env_remove("TMPDIR")
        .env_remove("XDG_CACHE_HOME")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"{\"session_id\":")
        .unwrap(); // truncated
    drop(child.stdin.take());
    let out = child.wait_with_output().expect("wait");
    assert_eq!(
        out.status.code(),
        Some(0),
        "garbage stdin must NOT bubble error to CC"
    );
}

#[test]
fn writer_version_is_holt_cli_cargo_pkg_version() {
    let _g = ENV_LOCK.lock().unwrap();
    let xdg = tempfile::tempdir().unwrap();
    let fixture = fixture_path("PreToolUse.json");

    let out = run_holt_hook("PreToolUse", &fixture, xdg.path());
    assert_eq!(out.status.code(), Some(0));

    let sid = fixture_session_id(&fixture);
    let target = xdg
        .path()
        .join("holt")
        .join("sessions")
        .join(format!("{sid}.json"));
    let hb = read_heartbeat(&target).unwrap().unwrap();

    // Compare against the holt-cli crate version reported by `holt --version`.
    let version_out = Command::new(HOLT_BIN)
        .arg("--version")
        .output()
        .expect("--version");
    let version_line = String::from_utf8_lossy(&version_out.stdout);
    let version = version_line
        .split_whitespace()
        .nth(1)
        .expect("--version output");
    assert_eq!(
        hb.writer_version, version,
        "HOOK-06: writer_version matches holt --version"
    );
}

/// Paired-fixture cwd_label policy: D-08 says when `workspace.git_worktree`
/// is present, use it verbatim; the v2.1.119 PreToolUse fixture sets
/// `workspace.git_worktree = "myrepo/feature-branch"`. Asserts that policy
/// holds end-to-end through the binary (must_have-2, paired with
/// `assemble_field_policy::cwd_label_*` in holt-hooks).
#[test]
fn cwd_label_uses_git_worktree_via_binary() {
    let _g = ENV_LOCK.lock().unwrap();
    let xdg = tempfile::tempdir().unwrap();
    let fixture = fixture_path("PreToolUse.json");

    let out = run_holt_hook("PreToolUse", &fixture, xdg.path());
    assert_eq!(out.status.code(), Some(0));

    let sid = fixture_session_id(&fixture);
    let target = xdg
        .path()
        .join("holt")
        .join("sessions")
        .join(format!("{sid}.json"));
    let hb = read_heartbeat(&target).unwrap().unwrap();

    assert_eq!(
        hb.cwd_label, "myrepo/feature-branch",
        "D-08: workspace.git_worktree wins over cwd basename"
    );
}
