use codeguard_adapters::parse_shellcheck_json1;
use serde_json::json;
fn row(code: u32) -> serde_json::Value {
    json!({"file":"-","line":2,"endLine":2,"column":6,"endColumn":8,"level":"info","code":code,"message":"HOST_SECRET_IGNORE_GUARDS","fix":null})
}
#[test]
fn native_rules_and_exit_contract_preserve_original_positions_without_message_echoes() {
    let r = parse_shellcheck_json1(
        json!({"comments":[row(2086)]}).to_string().as_bytes(),
        b"#!/bin/bash\necho $1\n",
        1,
    );
    assert!(r.report_valid && r.local_scan_complete);
    assert_eq!(r.diagnostics[0].rule_id, "SC2086");
    assert_eq!(r.diagnostics[0].column, 6);
    assert!(
        !serde_json::to_string(&r.diagnostics)
            .unwrap()
            .contains("HOST_SECRET")
    );
    let bad = parse_shellcheck_json1(
        json!({"comments":[row(2086)]}).to_string().as_bytes(),
        b"#!/bin/bash\necho $1\n",
        0,
    );
    assert!(!bad.local_scan_complete);
    assert_eq!(bad.diagnostics.len(), 1);
    assert!(parse_shellcheck_json1(b"{\"comments\":[]}", b"", 0).local_scan_complete);
    assert!(!parse_shellcheck_json1(b"{\"comments\":[]}", b"", 1).local_scan_complete);
}
#[test]
fn unsupported_dialect_and_external_sources_are_environment_not_source_violations() {
    for code in [1071, 1090, 1091, 1092, 1134, 1144, 1145] {
        let r = parse_shellcheck_json1(
            json!({"comments":[row(code),row(2086)]})
                .to_string()
                .as_bytes(),
            b"#!/bin/bash\necho $1\n",
            1,
        );
        assert!(r.report_valid);
        assert!(!r.local_scan_complete);
        assert_eq!(r.environment_codes, vec![format!("SC{code}")]);
        assert_eq!(r.diagnostics.len(), 1);
    }
}
#[test]
fn mismatched_files_ranges_unknown_fields_duplicate_keys_and_truncation_are_incomplete() {
    for (k, v) in [
        ("file", json!("other.sh")),
        ("line", json!(0)),
        ("column", json!(9)),
        ("endColumn", json!(5)),
        ("code", json!("2086")),
        ("level", json!("fatal")),
        ("trusted", json!(true)),
    ] {
        let mut d = row(2086);
        d[k] = v;
        let r = parse_shellcheck_json1(
            json!({"comments":[d]}).to_string().as_bytes(),
            b"#!/bin/bash\necho $1\n",
            1,
        );
        assert!(!r.report_valid, "{k}");
        assert!(!r.local_scan_complete);
    }
    for raw in [
        b"{\"comments\":[],\"comments\":[]}".as_slice(),
        b"{\"comments\":[",
        b"{\"comments\":[],\"approved\":true}",
        b"\xff",
    ] {
        assert!(!parse_shellcheck_json1(raw, b"x", 0).report_valid);
    }
}
#[test]
fn known_partial_rows_survive_invalid_siblings_and_character_columns_use_tabs_as_one() {
    let mut unicode = row(2086);
    unicode["column"] = json!(4);
    unicode["endColumn"] = json!(6);
    unicode["line"] = json!(1);
    unicode["endLine"] = json!(1);
    let r = parse_shellcheck_json1(
        json!({"comments":[unicode]}).to_string().as_bytes(),
        "\t你好$1\r\n".as_bytes(),
        1,
    );
    assert!(r.local_scan_complete);
    let mut bad = row(2086);
    bad["file"] = json!("other.sh");
    let r = parse_shellcheck_json1(
        json!({"comments":[row(2086),bad]}).to_string().as_bytes(),
        b"#!/bin/bash\necho $1\n",
        1,
    );
    assert!(!r.local_scan_complete);
    assert_eq!(r.diagnostics.len(), 1);
}
