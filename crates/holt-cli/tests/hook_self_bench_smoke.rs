//! D-15 hook self-bench JSON shape check.
//!
//! Mirrors `self_bench_smoke.rs` (Phase 1) — same JSON shape, same 20_000us
//! budget on Linux/macOS. Runs `holt --self-bench-hook PreToolUse --json
//! --iterations 30` and asserts the output shape + PASS gate.

use std::process::Command;

const HOLT_BIN: &str = env!("CARGO_BIN_EXE_holt");

#[test]
fn hook_self_bench_json_has_expected_shape() {
    let out = Command::new(HOLT_BIN)
        .arg("--self-bench-hook")
        .arg("PreToolUse")
        .arg("--json")
        .arg("--iterations")
        .arg("30")
        .output()
        .expect("run --self-bench-hook");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("--self-bench-hook --json must emit valid JSON. err={e} stdout={stdout}")
    });

    for field in [
        "iterations",
        "overhead_p50_us",
        "overhead_p95_us",
        "overhead_p99_us",
        "budget_p95_us",
        "passed",
    ] {
        assert!(v.get(field).is_some(), "missing field {field} in {v}");
    }

    assert!(v["iterations"].as_u64().unwrap() >= 30);

    // On the v0.1 platform tier (Unix tier-1), we ASSERT PASS. On Windows we
    // accept FAIL because Defender / spawn cost may exceed 40ms in CI; we
    // still verify the JSON shape.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let p95 = v["overhead_p95_us"].as_u64().unwrap();
        let budget = v["budget_p95_us"].as_u64().unwrap();
        assert_eq!(budget, 20_000, "D-15: budget_p95_us must be 20000 on Unix");
        assert!(
            v["passed"].as_bool().unwrap_or(false),
            "D-15: hook self-bench FAIL: p95 {p95}us > budget {budget}us. \
             Check release profile (D-04) and dep audit."
        );
        assert_eq!(out.status.code(), Some(0), "D-15: PASS expected exit 0");
    }
}

#[test]
fn hook_self_bench_human_output_shows_pass_or_fail() {
    let out = Command::new(HOLT_BIN)
        .arg("--self-bench-hook")
        .arg("PreToolUse")
        .arg("--iterations")
        .arg("10")
        .output()
        .expect("run --self-bench-hook");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Phase 1 print_human emits a "PASS:" or "FAIL:" line; we reuse it.
    assert!(
        stdout.contains("PASS") || stdout.contains("FAIL"),
        "human output must indicate PASS/FAIL status; got: {stdout}"
    );
}
