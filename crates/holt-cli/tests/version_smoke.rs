//! D-13 smoke: assert `holt --version` stdout starts with `concat!("holt ",
//! env!("CARGO_PKG_VERSION"))`. Guards a forgotten Cargo.toml version bump.
//! Plan 04-02 layers the D-14 release-workflow tag-parity check on top.

use std::process::Command;

#[test]
fn version_starts_with_holt_and_pkg_version() {
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
    let expected_prefix = concat!("holt ", env!("CARGO_PKG_VERSION"));
    assert!(
        stdout.starts_with(expected_prefix),
        "expected stdout to start with {expected_prefix:?}, got: {stdout:?}"
    );
}
