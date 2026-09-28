use codeguard_cli::corpus::{validate_corpus, validate_corpus_document};
use serde_json::{Value, json};
use std::path::Path;

fn locations() -> (std::path::PathBuf, std::path::PathBuf) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    (
        root.join("tests/fixtures/corpus/oracle.json"),
        root.join("../codeguard-plugin"),
    )
}

#[test]
fn current_corpus_keeps_pending_samples_separate() {
    let (index, plugin) = locations();
    let counts = validate_corpus(&index, &plugin).expect("固定语料应有效");
    assert_eq!(counts.accepted, 5);
    assert_eq!(counts.pending_reproduction, 3);
    assert_eq!(counts.disputed, 0);
}

#[test]
fn changed_source_digest_is_rejected() {
    let (index, plugin) = locations();
    let mut document: Value =
        serde_json::from_slice(&std::fs::read(&index).expect("oracle")).expect("JSON");
    document["cases"][0]["input"]["sha256"] = json!("0".repeat(64));
    assert!(validate_corpus_document(&document, index.parent().unwrap(), &plugin).is_err());
}

#[test]
fn unpinned_tool_cannot_be_accepted() {
    let (index, plugin) = locations();
    let mut document: Value =
        serde_json::from_slice(&std::fs::read(&index).expect("oracle")).expect("JSON");
    document["cases"][0]["tool"]["binary_sha256"] = Value::Null;
    assert!(validate_corpus_document(&document, index.parent().unwrap(), &plugin).is_err());
}

#[test]
fn unreviewed_sample_cannot_enter_holdout() {
    let (index, plugin) = locations();
    let mut document: Value =
        serde_json::from_slice(&std::fs::read(&index).expect("oracle")).expect("JSON");
    document["cases"][5]["cohort"] = json!("real_project_holdout");
    assert!(validate_corpus_document(&document, index.parent().unwrap(), &plugin).is_err());
}
