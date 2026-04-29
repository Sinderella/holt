//! must_have-1, -2, -3 owned by plan 03-02:
//!   1. Clean fixture → expected fixture byte-equal; `.holt.bak` byte-equal to pre-merge.
//!   2. Line-comments + key-order preserved end-to-end via the release binary.
//!   3. `--dry-run` + `--print` exit 0, do not mutate, produce the documented stdout shapes.
//!
//! Plus D-13 (`--help` UX ≤40 lines + mentions `.holt.bak`), D-16 (mutual exclusion of
//! `--dry-run` + `--print`), and D-17 (<500ms release / <800ms debug wall-clock budget).
//!
//! Tests use `HOME=$tempdir` override so the developer's real `~/.claude/settings.json`
//! is never touched (per CLAUDE.md project conventions). The test binary `holt` is
//! the cargo-built artifact for the current profile; the release-only D-17 ceiling
//! is relaxed to 800ms when `cfg!(debug_assertions)` is true (cargo test default).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use tempfile::tempdir;

fn holt_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_holt"))
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/settings")
        .join(name)
}

fn write_settings(home: &Path, name: &str) -> PathBuf {
    let claude = home.join(".claude");
    fs::create_dir_all(&claude).unwrap();
    let dst = claude.join("settings.json");
    fs::copy(fixture_path(name), &dst).unwrap();
    dst
}

#[test]
fn must_have_1_clean_fixture_byte_equal_and_bak_present() {
    let dir = tempdir().unwrap();
    let home = dir.path().to_path_buf();
    let settings = write_settings(&home, "clean.input.json");
    let pre_bytes = fs::read(&settings).unwrap();

    let out = Command::new(holt_binary())
        .env("HOME", &home)
        .arg("install-hooks")
        .output()
        .expect("spawn holt install-hooks");
    assert!(
        out.status.success(),
        "exit code: {:?}; stderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );

    let post = fs::read_to_string(&settings).unwrap();
    let expected = fs::read_to_string(fixture_path("clean.expected.json")).unwrap();
    assert_eq!(post, expected, "post-merge bytes != clean.expected.json");

    let bak = home.join(".claude/settings.json.holt.bak");
    assert!(bak.exists(), ".holt.bak not created");
    let bak_bytes = fs::read(&bak).unwrap();
    assert_eq!(
        bak_bytes, pre_bytes,
        ".holt.bak does not equal pre-merge bytes"
    );
}

#[test]
fn must_have_2_line_comments_and_key_order_preserved() {
    let dir = tempdir().unwrap();
    let home = dir.path().to_path_buf();
    let settings = write_settings(&home, "line_comments.input.json");
    let pre = fs::read_to_string(&settings).unwrap();

    let out = Command::new(holt_binary())
        .env("HOME", &home)
        .arg("install-hooks")
        .output()
        .expect("spawn holt install-hooks");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let post = fs::read_to_string(&settings).unwrap();
    // Comments preserved verbatim:
    for needle in ["// User's settings file", "// my hook"] {
        assert!(
            post.contains(needle),
            "missing comment `{needle}` in:\n{post}"
        );
    }

    // Key order check: every quoted key in the input file must appear in the
    // output at a byte position not less than the previous key's position.
    // Loose extraction — we only need to verify monotonicity of the sequence,
    // and the input is small, so any quoted-key pattern survives.
    let pre_keys: Vec<String> = extract_quoted_keys(&pre);
    let mut prev_pos = 0usize;
    for k in &pre_keys {
        if let Some(pos) = post.find(&format!("\"{k}\"")) {
            assert!(
                pos >= prev_pos,
                "key order violated: `{k}` at {pos} appeared before previous key at {prev_pos}"
            );
            prev_pos = pos;
        }
    }
}

fn extract_quoted_keys(s: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            // start of a quoted token
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'"' {
                if bytes[j] == b'\\' && j + 1 < bytes.len() {
                    j += 2;
                    continue;
                }
                j += 1;
            }
            if j >= bytes.len() {
                break;
            }
            let token = &s[start..j];
            // skip past closing quote, then skip whitespace, look for ':'
            let mut k = j + 1;
            while k < bytes.len() && bytes[k].is_ascii_whitespace() {
                k += 1;
            }
            if k < bytes.len() && bytes[k] == b':' {
                keys.push(token.to_string());
            }
            i = j + 1;
        } else {
            i += 1;
        }
    }
    keys
}

#[test]
fn must_have_3_dry_run_does_not_mutate_and_prints_diff() {
    let dir = tempdir().unwrap();
    let home = dir.path().to_path_buf();
    let settings = write_settings(&home, "clean.input.json");
    let mtime_before = fs::metadata(&settings).unwrap().modified().unwrap();

    let out = Command::new(holt_binary())
        .env("HOME", &home)
        .arg("install-hooks")
        .arg("--dry-run")
        .output()
        .expect("spawn holt install-hooks --dry-run");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let mtime_after = fs::metadata(&settings).unwrap().modified().unwrap();
    assert_eq!(
        mtime_before, mtime_after,
        "settings.json mtime changed under --dry-run"
    );

    let bak = home.join(".claude/settings.json.holt.bak");
    assert!(
        !bak.exists(),
        ".holt.bak created under --dry-run (must not be)"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--- "),
        "diff missing `--- ` header: {stdout}"
    );
    assert!(
        stdout.contains("+++ "),
        "diff missing `+++ ` header: {stdout}"
    );
    assert!(
        stdout.contains("\n+"),
        "diff missing `+`-prefixed lines: {stdout}"
    );
}

#[test]
fn must_have_3_print_does_not_mutate_and_emits_snippet() {
    let dir = tempdir().unwrap();
    let home = dir.path().to_path_buf();
    let settings = write_settings(&home, "clean.input.json");
    let mtime_before = fs::metadata(&settings).unwrap().modified().unwrap();

    let out = Command::new(holt_binary())
        .env("HOME", &home)
        .arg("install-hooks")
        .arg("--print")
        .output()
        .expect("spawn holt install-hooks --print");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let mtime_after = fs::metadata(&settings).unwrap().modified().unwrap();
    assert_eq!(
        mtime_before, mtime_after,
        "settings.json mtime changed under --print"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"command\": \"holt hook PreToolUse\""),
        "snippet missing PreToolUse command: {stdout}"
    );
    assert!(
        stdout.contains("\"PreToolUse\""),
        "snippet missing event key: {stdout}"
    );
    // 2-space-indent shape: top-level event keys begin at column 2.
    assert!(
        stdout.contains("\n  \"PreToolUse\""),
        "snippet missing 2-space indent: {stdout}"
    );
}

#[test]
fn dry_run_and_print_are_mutually_exclusive() {
    let dir = tempdir().unwrap();
    let home = dir.path().to_path_buf();
    write_settings(&home, "clean.input.json");

    let out = Command::new(holt_binary())
        .env("HOME", &home)
        .arg("install-hooks")
        .arg("--dry-run")
        .arg("--print")
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--dry-run") && stderr.contains("--print"),
        "stderr does not name both flags: {stderr}"
    );
}

#[test]
fn d17_completes_within_500ms_in_release_or_800ms_in_debug() {
    let dir = tempdir().unwrap();
    let home = dir.path().to_path_buf();
    write_settings(&home, "clean.input.json");

    let start = Instant::now();
    let out = Command::new(holt_binary())
        .env("HOME", &home)
        .arg("install-hooks")
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let elapsed = start.elapsed();
    // Release build: D-17 budget is <500ms. Debug build (cargo test default):
    // 800ms slack because clap parsing + jsonc-parser init are slower without
    // LTO / strip on debug.
    let limit_ms: u64 = if cfg!(debug_assertions) { 800 } else { 500 };
    assert!(
        elapsed.as_millis() < u128::from(limit_ms),
        "install-hooks took {}ms (limit {}ms)",
        elapsed.as_millis(),
        limit_ms
    );
}

#[test]
fn help_text_mentions_dry_run_print_and_holt_bak_in_at_most_40_lines() {
    let out = Command::new(holt_binary())
        .arg("install-hooks")
        .arg("--help")
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("--dry-run"), "help missing --dry-run");
    assert!(stdout.contains("--print"), "help missing --print");
    assert!(
        stdout.contains(".holt.bak"),
        "help missing .holt.bak (D-13)"
    );
    let line_count = stdout.lines().count();
    assert!(
        line_count <= 40,
        "help is {line_count} lines, D-13 mandates ≤40"
    );
}
