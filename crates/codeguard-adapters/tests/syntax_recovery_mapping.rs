use codeguard_adapters::map_syntax_recovery;
use codeguard_core::SyntaxRecoveryAnchor;

#[test]
fn maps_utf8_and_crlf_without_losing_original_byte_anchor() {
    let source = "class A {\r\n  String s = \"中\";\r\n";
    let start_byte = source.find(";\r\n").unwrap() + 1;
    let row_start = source.find('\n').unwrap() + 1;
    let anchor = SyntaxRecoveryAnchor {
        kind: "MISSING",
        group_id: 7,
        syntax_kind: "}".into(),
        start_byte,
        end_byte: start_byte,
        start_row: 1,
        start_column_byte: start_byte - row_start,
        end_row: 1,
        end_column_byte: start_byte - row_start,
    };
    let mapped = map_syntax_recovery(source.as_bytes(), &anchor).unwrap();
    assert_eq!(mapped.start_byte, start_byte);
    assert_eq!(mapped.start_line, 2);
    assert_eq!(
        mapped.start_column_scalar,
        "  String s = \"中\";".chars().count() + 1
    );
    assert_eq!(mapped.kind, "MISSING");
    assert_eq!(mapped.group_id, 7);
    assert_eq!(mapped.syntax_kind, "}");
}

#[test]
fn invalid_encoding_boundaries_and_mismatched_points_are_incomplete() {
    let source = "a中";
    let anchor = SyntaxRecoveryAnchor {
        kind: "ERROR",
        group_id: 1,
        syntax_kind: "ERROR".into(),
        start_byte: 1,
        end_byte: 4,
        start_row: 0,
        start_column_byte: 1,
        end_row: 0,
        end_column_byte: 4,
    };
    assert!(map_syntax_recovery(source.as_bytes(), &anchor).is_ok());
    let mut invalid = anchor.clone();
    invalid.start_byte = 2;
    assert!(map_syntax_recovery(source.as_bytes(), &invalid).is_err());
    let mut invalid = anchor.clone();
    invalid.start_row = 1;
    assert!(map_syntax_recovery(source.as_bytes(), &invalid).is_err());
    let mut invalid = anchor.clone();
    invalid.end_byte = source.len() + 1;
    assert!(map_syntax_recovery(source.as_bytes(), &invalid).is_err());
    let mut invalid = anchor.clone();
    invalid.kind = "CLEAN";
    assert!(map_syntax_recovery(source.as_bytes(), &invalid).is_err());
    let mut invalid = anchor.clone();
    invalid.group_id = 0;
    assert!(map_syntax_recovery(source.as_bytes(), &invalid).is_err());
    assert!(map_syntax_recovery(b"\xff", &anchor).is_err());
}
