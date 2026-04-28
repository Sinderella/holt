//! D-15 / CORE-10 / C2 enforcement.
//!
//! Asserts: no path from `holt-render` package to `holt-supervisor` package in the
//! resolved cargo metadata graph. Adding `holt-supervisor` to `holt-render`'s
//! Cargo.toml MUST fail this test.
//!
//! Strategy: shell out to `cargo metadata --format-version 1`, parse with serde_json
//! (no extra dev-dependency), BFS from the holt-render node.

use std::collections::{HashMap, HashSet, VecDeque};
use std::process::Command;

/// Parse a Cargo package ID into a bare crate name.
///
/// Handles the three observed shapes:
///   - `path+file:///abs/path/to/crate-x#0.1.0`             → `crate-x`
///   - `path+file:///abs/path#crate-x@0.1.0`                → `crate-x`
///   - `registry+https://.../index#crate-x@1.2.3`           → `crate-x`
///   - legacy whitespace form: `crate-x 0.1.0 (path+...)`   → `crate-x`
fn parse_package_name(id: &str) -> String {
    // Legacy format: `name version (source)`.
    if let Some(first_space) = id.find(' ') {
        if !id.starts_with("path+") && !id.starts_with("registry+") && !id.starts_with("git+") {
            return id[..first_space].to_string();
        }
    }

    // Modern PkgIdSpec: split on `#`, then handle `@` or trailing path segment.
    if let Some(after_hash) = id.split_once('#').map(|(_, t)| t) {
        // `name@version` form
        if let Some((name, _ver)) = after_hash.split_once('@') {
            return name.to_string();
        }
        // `<version>` only — name is the last URL segment before `#`.
        let before_hash = id.split_once('#').map(|(h, _)| h).unwrap_or("");
        let last_segment = before_hash
            .rsplit('/')
            .find(|s| !s.is_empty())
            .unwrap_or("");
        return last_segment.to_string();
    }

    id.to_string()
}

#[test]
fn holt_render_does_not_depend_on_holt_supervisor() {
    let out = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1"])
        .output()
        .expect("cargo metadata --format-version 1 must succeed");
    assert!(
        out.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("cargo metadata is valid JSON");

    let nodes = v
        .pointer("/resolve/nodes")
        .and_then(|n| n.as_array())
        .expect("resolve.nodes must be present in cargo metadata");

    let mut deps: HashMap<String, Vec<String>> = HashMap::new();
    let mut name_to_id: HashMap<String, String> = HashMap::new();

    for n in nodes {
        let id = n["id"].as_str().expect("node.id is a string").to_string();
        // Cargo 1.77+ Package ID Spec: `<source>#<name>@<version>` for workspace
        // and registry crates, or `<source>#<version>` when the crate name equals
        // the last URL segment. Older formats used space-separated tokens.
        // Extract the bare package name defensively.
        let name = parse_package_name(&id);
        name_to_id.insert(name, id.clone());

        let dep_ids: Vec<String> = n["deps"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|d| d["pkg"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        deps.insert(id, dep_ids);
    }

    let render_id = name_to_id
        .get("holt-render")
        .expect("holt-render must be in the workspace metadata");
    let supervisor_id = name_to_id
        .get("holt-supervisor")
        .expect("holt-supervisor must be in the workspace metadata");

    // BFS from holt-render. If we ever reach holt-supervisor, fail loudly.
    let mut seen: HashSet<String> = HashSet::new();
    let mut q: VecDeque<String> = VecDeque::new();
    q.push_back(render_id.clone());
    seen.insert(render_id.clone());

    while let Some(cur) = q.pop_front() {
        let Some(neighbors) = deps.get(&cur) else {
            continue;
        };
        for d in neighbors {
            assert_ne!(
                d, supervisor_id,
                "C2 VIOLATED: holt-render has a dependency path to holt-supervisor.\n\
                 The chain reached holt-supervisor via {cur}.\n\
                 Render path (20ms budget) MUST NOT depend on supervisor (unbounded \
                 user-script runtime). See CLAUDE.md hard constraint C2."
            );
            if seen.insert(d.clone()) {
                q.push_back(d.clone());
            }
        }
    }
}
