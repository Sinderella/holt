//! Test-only driver — fork target for `tests/sigkill_atomicity.rs`. Reads a
//! fixture path from env, calls `handle_event`, exits 0. NOT an end-user
//! binary; gated by `test = false, bench = false` in Cargo.toml so it never
//! lands in `cargo build` artifacts.

use holt_hooks::{Env, HookEvent, handle_event};

fn main() {
    let fixture =
        std::env::var("HOLT_HOOKS_TEST_FIXTURE").expect("driver: HOLT_HOOKS_TEST_FIXTURE not set");
    let bytes = std::fs::read(&fixture).expect("driver: fixture not readable");
    let env = Env {
        writer_version: "sigkill-driver-0.0.0",
        pid: std::process::id(),
        now_iso: "2026-04-28T10:00:00Z".to_string(),
    };
    let _ = handle_event(HookEvent::PreToolUse, &bytes, &env);
    std::process::exit(0);
}
