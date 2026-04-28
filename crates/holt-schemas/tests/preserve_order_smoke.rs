//! CR-01 regression: serde_json must be built with the `preserve_order` feature.
//!
//! Without `preserve_order`, `serde_json::Value` deserializes object keys into a
//! BTreeMap (alphabetical). With it, keys round-trip in source insertion order.
//! ccstatusline and Phase 2 hook code rely on the byte-stable ordering when
//! re-serializing CC stdin envelopes; this test fails at compile-or-run if the
//! workspace ever drops the feature flag.

#[test]
fn serde_json_preserves_object_key_order() {
    let src = r#"{"b":1,"a":2,"zzz":3,"alpha":4}"#;
    let v: serde_json::Value = serde_json::from_str(src).expect("valid json");
    let round_tripped = serde_json::to_string(&v).expect("re-serialize");
    assert_eq!(
        round_tripped, src,
        "serde_json `preserve_order` feature is missing — keys re-ordered"
    );
}
