use codeguard_adapters::parse_cargo_rustdoc_json;
use codeguard_adapters::rustdoc_finding_record;
use serde_json::json;

fn record(source: &[u8], start: u64, end: u64) -> serde_json::Value {
    let prefix = std::str::from_utf8(&source[..start as usize]).unwrap();
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().unwrap().chars().count() + 1;
    let rows = format!(
        "{}\n{}",
        json!({"reason":"compiler-message","package_id":"sample",
        "manifest_path":"/project/Cargo.toml","target":{"kind":["lib"],"src_path":"/project/src/lib.rs"},
        "message":{"code":{"code":"missing_docs"},"level":"warning",
            "spans":[{"file_name":"src/lib.rs","line_start":line,"column_start":column,"byte_start":start,"byte_end":end,"is_primary":true}]}}),
        json!({"reason":"build-finished","success":true})
    );
    let parsed = parse_cargo_rustdoc_json(rows.as_bytes());
    assert_eq!(parsed.issue, None);
    rustdoc_finding_record(&parsed.findings[0], source, "src/lib.rs").unwrap()
}

#[test]
fn a_line_shift_does_not_change_identity_of_the_same_native_declaration_span() {
    let before = record(b"pub fn answer() {}\n", 0, 15);
    let after = record(b"// unrelated\npub fn answer() {}\n", 13, 28);
    assert_eq!(before["finding_id"], after["finding_id"]);
    assert_ne!(before["source_sha256"], after["source_sha256"]);
}

#[test]
fn crate_and_function_spans_on_the_same_line_do_not_borrow_each_others_identity() {
    let source = b"pub fn answer() {}\n";
    let crate_span = record(source, 0, source.len() as u64);
    let function_span = record(source, 0, 15);
    assert_ne!(crate_span["finding_id"], function_span["finding_id"]);
}

#[test]
fn byte_ranges_must_match_real_source_and_utf8_boundaries() {
    let mut rows = json!({"reason":"compiler-message","package_id":"sample","manifest_path":"/project/Cargo.toml",
        "target":{"kind":["lib"],"src_path":"/project/src/lib.rs"},
        "message":{"code":{"code":"missing_docs"},"level":"warning","spans":[{"file_name":"src/lib.rs","line_start":1,"column_start":1,"byte_start":0,"byte_end":99,"is_primary":true}]}});
    let parsed = parse_cargo_rustdoc_json(rows.to_string().as_bytes());
    assert!(rustdoc_finding_record(&parsed.findings[0], b"short", "src/lib.rs").is_err());
    rows["message"]["spans"][0]["byte_end"] = json!(1);
    let parsed = parse_cargo_rustdoc_json(rows.to_string().as_bytes());
    assert!(rustdoc_finding_record(&parsed.findings[0], "文档".as_bytes(), "src/lib.rs").is_err());
    let mut wrong_location = parsed.findings[0].clone();
    wrong_location.line = 99;
    assert_eq!(
        rustdoc_finding_record(&wrong_location, b"short", "src/lib.rs").unwrap_err(),
        "native_finding_location_mismatch"
    );
}
