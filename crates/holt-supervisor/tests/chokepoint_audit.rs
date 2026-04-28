//! C1 chokepoint audit: there must be EXACTLY ONE textual occurrence of
//! `.wrap(ProcessGroup::leader())` in the entire `holt-supervisor` crate's
//! `src/`.
//!
//! Adding a second one anywhere bypasses the C1 invariant (always pipe stdio
//! first) and is an audit hazard. This test fires on every `cargo test`.
//!
//! Doc-comment lines (Rust line comments — `//`, `///`, `//!`) are filtered
//! out so the module-level docs that mention the call name don't accidentally
//! count.

use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn only_one_wrap_call_site_in_supervisor_crate() {
    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let needle = ".wrap(ProcessGroup::leader())";

    let mut total = 0usize;
    let mut hits: Vec<(PathBuf, usize, String)> = Vec::new();

    for entry in walk(&crate_src) {
        if entry.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let text = fs::read_to_string(&entry).unwrap_or_default();
        for (i, line) in text.lines().enumerate() {
            // Strip Rust line comments so doc-comments don't count.
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            let count = line.matches(needle).count();
            if count > 0 {
                total += count;
                hits.push((entry.clone(), i + 1, line.to_string()));
            }
        }
    }

    assert_eq!(
        total,
        1,
        "C1 chokepoint violated: expected exactly 1 .wrap(ProcessGroup::leader()) \
         call site across crates/holt-supervisor/src/, found {total}.\n\
         Every supervised spawn must go through Supervisor::wrap_and_run.\n\
         Hits:\n{}",
        hits.iter()
            .map(|(p, ln, l)| format!("  {}:{ln}: {}", p.display(), l.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}
