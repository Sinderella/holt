//! D-11 hand-rolled unified diff for `holt install-hooks --dry-run`.
//!
//! Strategy: line-based naive diff using common-prefix + common-suffix
//! detection (no LCS). Both sides are small (<200 lines typically). Output
//! format follows the conventional unified-diff shape:
//!
//!     --- a/settings.json
//!     +++ b/settings.json
//!     @@ -lA,nA +lB,nB @@
//!      <context line>
//!     -<old line>
//!     +<new line>
//!      <context line>
//!
//! Implementation: split both into `Vec<&str>` by '\n'; find longest-common
//! prefix and longest-common suffix (cheap O(n)); emit a single hunk for the
//! middle. Sufficient for D-11 — `holt install-hooks` is appending entries
//! to settings.json, which is structurally a contiguous diff at the bottom
//! of the file.
//!
//! Why not the `similar` crate: extra release-binary dep for a feature that
//! is shown to the user only when they pass `--dry-run`. The diff is small
//! and contiguous; LCS-free is fine.

pub fn unified_diff(a: &str, b: &str, a_label: &str, b_label: &str) -> String {
    let a_lines: Vec<&str> = a.split('\n').collect();
    let b_lines: Vec<&str> = b.split('\n').collect();

    // Common prefix length.
    let prefix_len = a_lines
        .iter()
        .zip(b_lines.iter())
        .take_while(|(x, y)| x == y)
        .count();
    // Common suffix length, bounded so prefix + suffix never exceed either side.
    let max_suffix =
        (a_lines.len().saturating_sub(prefix_len)).min(b_lines.len().saturating_sub(prefix_len));
    let suffix_len = (0..max_suffix)
        .take_while(|i| a_lines[a_lines.len() - 1 - i] == b_lines[b_lines.len() - 1 - i])
        .count();

    let a_hunk = &a_lines[prefix_len..a_lines.len() - suffix_len];
    let b_hunk = &b_lines[prefix_len..b_lines.len() - suffix_len];

    let mut out = String::new();
    out.push_str(&format!("--- {a_label}\n"));
    out.push_str(&format!("+++ {b_label}\n"));

    if a_hunk.is_empty() && b_hunk.is_empty() {
        // No content delta — emit nothing after the headers (callers can
        // grep for `^---` to know the diff was attempted).
        return out;
    }

    // Up to 3 lines of leading context (or fewer if the file is short).
    let ctx_pre = prefix_len.saturating_sub(3);
    let ctx_pre_lines = &a_lines[ctx_pre..prefix_len];
    let ctx_post_count = suffix_len.min(3);
    let ctx_post_start = a_lines.len() - suffix_len;
    let ctx_post_lines = &a_lines[ctx_post_start..ctx_post_start + ctx_post_count];

    let a_start = ctx_pre + 1; // 1-indexed per unified-diff convention
    let b_start = ctx_pre + 1;
    let a_count = ctx_pre_lines.len() + a_hunk.len() + ctx_post_lines.len();
    let b_count = ctx_pre_lines.len() + b_hunk.len() + ctx_post_lines.len();

    out.push_str(&format!(
        "@@ -{a_start},{a_count} +{b_start},{b_count} @@\n"
    ));
    for ctx in ctx_pre_lines {
        out.push_str(&format!(" {ctx}\n"));
    }
    for old in a_hunk {
        out.push_str(&format!("-{old}\n"));
    }
    for new_l in b_hunk {
        out.push_str(&format!("+{new_l}\n"));
    }
    for ctx in ctx_post_lines {
        out.push_str(&format!(" {ctx}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appended_block_produces_minus_and_plus_markers() {
        let a = "line one\nline two\n";
        let b = "line one\nline two\nappended\n";
        let d = unified_diff(a, b, "a", "b");
        assert!(d.contains("--- a"));
        assert!(d.contains("+++ b"));
        assert!(d.contains("+appended"));
    }

    #[test]
    fn identical_inputs_produce_only_headers() {
        let a = "x\ny\n";
        let d = unified_diff(a, a, "a", "b");
        assert!(d.contains("--- a"));
        assert!(d.contains("+++ b"));
        // No `+`-prefixed or `-`-prefixed content lines (only the `+++` /
        // `---` headers, which are 3-char prefixes).
        for line in d.lines() {
            if line.starts_with("---") || line.starts_with("+++") {
                continue;
            }
            assert!(!line.starts_with('+'), "unexpected + line: {line}");
            assert!(!line.starts_with('-'), "unexpected - line: {line}");
        }
    }

    #[test]
    fn middle_replacement_emits_both_minus_and_plus() {
        let a = "a\nb\nc\nd\ne\n";
        let b = "a\nb\nX\nd\ne\n";
        let d = unified_diff(a, b, "a", "b");
        assert!(d.contains("-c"), "missing minus line in {d}");
        assert!(d.contains("+X"), "missing plus line in {d}");
    }
}
