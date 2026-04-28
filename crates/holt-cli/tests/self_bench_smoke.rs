//! D-14 / CORE-07 smoke: `holt --self-bench --json --iterations 10` returns valid JSON
//! with all expected fields and exits 0 (PASS) on macOS arm64 / Linux x86_64.

use std::process::Command;

#[test]
fn self_bench_json_has_expected_shape() {
    let exe = env!("CARGO_BIN_EXE_holt");
    let out = Command::new(exe)
        .args(["--self-bench", "--json", "--iterations", "10"])
        .output()
        .expect("spawn holt --self-bench");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("self-bench --json must emit valid JSON. err={e} stdout={stdout}")
    });

    for field in [
        "iterations",
        "overhead_p50_us",
        "overhead_p95_us",
        "overhead_p99_us",
        "budget_p95_us",
        "passed",
    ] {
        assert!(json.get(field).is_some(), "missing field {field} in {json}");
    }

    assert!(json["iterations"].as_u64().unwrap() >= 10);

    // On the v0.1 platform tier (Unix tier-1), we ASSERT PASS. On Windows we accept
    // FAIL because Defender / spawn cost may exceed 40ms in CI; we still verify the
    // JSON shape.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let p95 = json["overhead_p95_us"].as_u64().unwrap();
        let budget = json["budget_p95_us"].as_u64().unwrap();
        assert_eq!(
            budget, 20_000,
            "budget_p95_us must be 20000 on Unix per D-14"
        );
        assert!(
            json["passed"].as_bool().unwrap_or(false),
            "self-bench FAIL: p95 {p95}us > budget {budget}us. Check release profile (D-04) and dep audit."
        );
    }
}
