//! Smoke: `holt --version` exits 0 and prints a non-empty line containing 0.1.0.

use std::process::Command;

#[test]
fn version_prints_semver_and_exits_zero() {
    let exe = env!("CARGO_BIN_EXE_holt");
    let out = Command::new(exe)
        .arg("--version")
        .output()
        .expect("holt binary must be runnable");
    assert!(
        out.status.success(),
        "holt --version exited non-zero: {out:?}"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("0.1.0"),
        "expected version 0.1.0 in stdout, got: {stdout}"
    );
}
