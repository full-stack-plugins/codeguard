use codeguard_adapters::GoVetFinding;
use codeguard_adapters::go_finding_record;

fn finding(line: u32) -> GoVetFinding {
    GoVetFinding {
        package: "example.com/app".into(),
        native_rule_id: "printf".into(),
        path: "main.go".into(),
        line,
        column: 1,
        message: "printf argument type mismatch".into(),
    }
}

#[test]
fn line_shift_keeps_id_but_changes_source_identity() {
    let before = go_finding_record(".", &finding(2), b"package main\nbad()\n", 0).unwrap();
    let after = go_finding_record(".", &finding(3), b"package main\n\nbad()\n", 0).unwrap();
    assert_eq!(before["finding_id"], after["finding_id"]);
    assert_ne!(before["source_sha256"], after["source_sha256"]);
    assert_eq!(after["line"], 3);
}

#[test]
fn evidence_and_occurrence_cannot_collapse_distinct_findings() {
    let original = finding(2);
    let one = go_finding_record(".", &original, b"package main\nbad()\n", 0).unwrap();
    let two = go_finding_record(".", &original, b"package main\nbad()\n", 1).unwrap();
    assert_ne!(one["finding_id"], two["finding_id"]);
    let mut changed = original;
    changed.message = "different native diagnostic".into();
    let changed = go_finding_record(".", &changed, b"package main\nbad()\n", 0).unwrap();
    assert_ne!(one["finding_id"], changed["finding_id"]);
    let other_module =
        go_finding_record("nested", &finding(2), b"package main\nbad()\n", 0).unwrap();
    assert_ne!(one["finding_id"], other_module["finding_id"]);
    let changed_source =
        go_finding_record(".", &finding(2), b"package main\nother()\n", 0).unwrap();
    assert_ne!(one["finding_id"], changed_source["finding_id"]);
}

#[test]
fn invalid_native_position_does_not_produce_a_finding() {
    assert!(go_finding_record(".", &finding(3), b"package main\nbad()", 0).is_none());
    let mut invalid = finding(2);
    invalid.column = 100;
    assert!(go_finding_record(".", &invalid, b"package main\nbad()\n", 0).is_none());
}
