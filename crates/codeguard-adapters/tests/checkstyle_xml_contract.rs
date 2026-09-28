use codeguard_adapters::parse_checkstyle_xml;

fn report(body: &str) -> String {
    format!("<checkstyle version=\"10.0\">{body}</checkstyle>")
}

#[test]
fn native_fields_and_custom_rule_identity_are_preserved() {
    let xml = report(
        "<file name=\"src/Foo.java\"><error line=\"0\" severity=\"warning\" message=\"a &amp; b\" source=\"custom.DocCheck#publicApi\"/><error line=\"2\" column=\"3\" severity=\"info\" message=\"detail\" source=\"custom.DocCheck#internal\"/></file>",
    );
    let parsed = parse_checkstyle_xml(xml.as_bytes(), "10.0");
    assert!(parsed.report_valid);
    assert_eq!(parsed.files, ["src/Foo.java"]);
    assert_eq!(parsed.diagnostics.len(), 2);
    assert_eq!(parsed.diagnostics[0].source, "custom.DocCheck#publicApi");
    assert_eq!(parsed.diagnostics[0].message, "a & b");
    assert_eq!(parsed.diagnostics[0].line, 0);
    assert_eq!(parsed.diagnostics[0].column, None);
    assert_eq!(parsed.diagnostics[1].severity, "info");
    assert_eq!(parsed.diagnostics[1].column, Some(3));
    // 10.21.4 原生 XMLLogger 在有自定义 ID 时只输出 ID，禁止反推类名。
    let legacy = report(
        "<file name=\"Foo.java\"><error line=\"1\" severity=\"error\" message=\"missing doc\" source=\"publicType\"/></file>",
    );
    let parsed = parse_checkstyle_xml(legacy.as_bytes(), "10.0");
    assert!(parsed.report_valid);
    assert_eq!(parsed.diagnostics[0].source, "publicType");
}

#[test]
fn exceptions_do_not_become_source_violations_or_empty_success() {
    let xml = report(
        "<file name=\"Foo.java\"><exception><![CDATA[java.lang.RuntimeException: broken]]></exception></file>",
    );
    let parsed = parse_checkstyle_xml(xml.as_bytes(), "10.0");
    assert!(!parsed.report_valid);
    assert_eq!(parsed.processing_errors, 1);
    assert!(parsed.diagnostics.is_empty());
    let mixed = report(
        "<file name=\"Foo.java\"><error line=\"1\" severity=\"error\" message=\"missing doc\" source=\"DocCheck\"/><exception><![CDATA[broken]]></exception></file>",
    );
    let parsed = parse_checkstyle_xml(mixed.as_bytes(), "10.0");
    assert!(!parsed.report_valid);
    assert_eq!(parsed.processing_errors, 1);
    assert_eq!(parsed.diagnostics.len(), 1);
}

#[test]
fn valid_empty_file_is_a_report_observation_not_coverage_proof() {
    let parsed = parse_checkstyle_xml(report("<file name=\"Foo.java\"/>").as_bytes(), "10.0");
    assert!(parsed.report_valid);
    assert_eq!(parsed.files, ["Foo.java"]);
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn malformed_or_ambiguous_report_never_claims_valid() {
    let cases = [
        "<file name=\"Foo.java\"/><file name=\"Foo.java\"/>",
        "<file name=\"Foo.java\"><error line=\"1\" severity=\"fatal\" message=\"x\" source=\"Check\"/></file>",
        "<file name=\"Foo.java\"><error line=\"-1\" severity=\"error\" message=\"x\" source=\"Check\"/></file>",
        "<file name=\"Foo.java\"><error line=\"1\" column=\"0\" severity=\"error\" message=\"x\" source=\"Check\"/></file>",
        "<file name=\"Foo.java\"><error line=\"1\" severity=\"error\" message=\"x\" source=\"\"/></file>",
        "<file name=\"Foo.java\"><future/></file>",
        "<file name=\"Foo.java\">unexpected</file>",
        "<error line=\"1\" severity=\"error\" message=\"x\" source=\"Check\"/>",
        "<file name=\"Foo.java\" unknown=\"1\"/>",
    ];
    for body in cases {
        assert!(
            !parse_checkstyle_xml(report(body).as_bytes(), "10.0").report_valid,
            "{body}"
        );
    }
    for xml in [
        b"<checkstyle version=\"11.0\"/>".as_slice(),
        b"<!DOCTYPE checkstyle [<!ENTITY x 'bad'>]><checkstyle version=\"10.0\"/>",
        b"<checkstyle version=\"10.0\">",
        b"\xff",
    ] {
        assert!(!parse_checkstyle_xml(xml, "10.0").report_valid);
    }
}

#[test]
fn version_or_partial_structure_failure_keeps_valid_diagnostic_evidence() {
    let xml = report(
        "<file name=\"Foo.java\"><error line=\"1\" severity=\"error\" message=\"missing doc\" source=\"DocCheck\"/><future/></file>",
    );
    let parsed = parse_checkstyle_xml(xml.as_bytes(), "10.0");
    assert!(!parsed.report_valid);
    assert_eq!(parsed.diagnostics.len(), 1);
    let parsed = parse_checkstyle_xml(xml.as_bytes(), "11.0");
    assert!(!parsed.report_valid);
    assert_eq!(parsed.diagnostics.len(), 1);
}

#[test]
fn namespace_unknown_attributes_numeric_overflow_and_size_are_incomplete() {
    for xml in [
        "<checkstyle xmlns=\"urn:foreign\" version=\"10.0\"/>",
        "<checkstyle version=\"10.0\" unknown=\"true\"/>",
        "<checkstyle version=\"10.0\"><file name=\"Foo.java\"><error line=\"4294967296\" severity=\"error\" message=\"x\" source=\"Check\"/></file></checkstyle>",
    ] {
        assert!(!parse_checkstyle_xml(xml.as_bytes(), "10.0").report_valid);
    }
    let bytes = vec![b' '; 16 * 1024 * 1024 + 1];
    let parsed = parse_checkstyle_xml(&bytes, "10.0");
    assert!(!parsed.report_valid);
    assert_eq!(parsed.reason, Some("report_too_large"));
    assert!(!parse_checkstyle_xml(b"<checkstyle version=\"\"/>", "").report_valid);
}
