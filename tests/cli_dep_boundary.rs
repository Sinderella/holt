//! D-03 + D-05 / C4 enforcement.
//!
//! Asserts: no path from any non-`holt-cli` workspace package to either
//! `jsonc-parser` or `fs2` packages in the resolved cargo metadata graph.
//! Adding either dep to `holt-schemas`, `holt-supervisor`, `holt-hooks`,
//! `holt-orchestrator`, or `holt-render` MUST fail this test.
//!
//! Strategy mirrors `tests/architecture_dag.rs`: shell out to
//! `cargo metadata --format-version 1`, BFS from each non-holt-cli workspace
//! package, fail if either jsonc-parser or fs2 is reached.
//!
//! False-positive guard: also sanity-check that `holt-cli` CAN reach both
//! targets, so a regression that drops the deps does not accidentally pass.

use std::collections::{HashMap, HashSet, VecDeque};
use std::process::Command;

/// Parse a Cargo package ID into a bare crate name. Same impl as
/// `tests/architecture_dag.rs::parse_package_name` — kept as a sibling so the
/// two boundary tests are independent (no test-helper module to invalidate
/// either of them on refactor).
fn parse_package_name(id: &str) -> String {
    if let Some(first_space) = id.find(' ') {
        if !id.starts_with("path+") && !id.starts_with("registry+") && !id.starts_with("git+") {
            return id[..first_space].to_string();
        }
    }
    if let Some(after_hash) = id.split_once('#').map(|(_, t)| t) {
        if let Some((name, _ver)) = after_hash.split_once('@') {
            return name.to_string();
        }
        let before_hash = id.split_once('#').map(|(h, _)| h).unwrap_or("");
        let last_segment = before_hash
            .rsplit('/')
            .find(|s| !s.is_empty())
            .unwrap_or("");
        return last_segment.to_string();
    }
    id.to_string()
}

const FORBIDDEN_TARGETS: &[&str] = &["jsonc-parser", "fs2"];
const ALLOWED_CONSUMER: &str = "holt-cli";
const NON_CLI_WORKSPACE_CRATES: &[&str] = &[
    "holt-schemas",
    "holt-supervisor",
    "holt-hooks",
    "holt-orchestrator",
    "holt-render",
];

#[test]
fn jsonc_parser_and_fs2_are_only_reachable_from_holt_cli() {
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

    // Sanity-check: `holt-cli` MUST be reachable to either target. Otherwise
    // plan 03-01 stripped a dep and this test would silently pass for the
    // wrong reason. Guard against that false-positive negative.
    let cli_id = name_to_id
        .get(ALLOWED_CONSUMER)
        .unwrap_or_else(|| panic!("{ALLOWED_CONSUMER} not in workspace metadata"));
    for target in FORBIDDEN_TARGETS {
        let target_id = name_to_id.get(*target).unwrap_or_else(|| {
            panic!(
                "{target} package not present in resolved metadata — did plan 03-01 add the dep?"
            )
        });
        assert!(
            reachable(&deps, cli_id, target_id),
            "{ALLOWED_CONSUMER} should be able to reach {target}, but the dep edge is missing — plan 03-01 may have stripped it"
        );
    }

    // Main contract: every non-cli workspace crate must NOT be able to reach
    // either forbidden target. Adding `jsonc-parser` or `fs2` to any of those
    // crates' Cargo.toml will cause `cargo metadata` to surface a path here.
    for crate_name in NON_CLI_WORKSPACE_CRATES {
        let start_id = name_to_id
            .get(*crate_name)
            .unwrap_or_else(|| panic!("workspace crate {crate_name} missing from metadata"));
        for target in FORBIDDEN_TARGETS {
            let target_id = name_to_id.get(*target).unwrap();
            assert!(
                !reachable(&deps, start_id, target_id),
                "C4 VIOLATED: {crate_name} has a dependency path to {target}.\n\
                 Per CLAUDE.md hard constraint C4 + 03-CONTEXT.md D-03/D-05, both deps\n\
                 must live ONLY in crates/holt-cli/Cargo.toml. Move the dep back to\n\
                 holt-cli or refactor the consumer to avoid it."
            );
        }
    }
}

fn reachable(deps: &HashMap<String, Vec<String>>, start: &str, target: &str) -> bool {
    let mut seen: HashSet<String> = HashSet::new();
    let mut q: VecDeque<String> = VecDeque::new();
    q.push_back(start.to_string());
    seen.insert(start.to_string());
    while let Some(cur) = q.pop_front() {
        let Some(neighbors) = deps.get(&cur) else {
            continue;
        };
        for d in neighbors {
            if d == target {
                return true;
            }
            if seen.insert(d.clone()) {
                q.push_back(d.clone());
            }
        }
    }
    false
}
