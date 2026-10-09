use serde_json::{Value, json};
fn report() -> Value {
    json!({"version":"2.1.0","runs":[{"columnKind":"unicodeCodePoints","invocations":[{"executionSuccessful":false}],"artifacts":[{"location":{"uri":"file://","index":0}}],"tool":{"driver":{"name":"clang","version":"Apple clang version 21.0.0 (clang-2100.3.34.2)","rules":[{"id":"err_expected_expression"}]}},"results":[{"level":"error","ruleId":"err_expected_expression","ruleIndex":0,"locations":[{"physicalLocation":{"artifactLocation":{"uri":"file://","index":0},"region":{"startLine":1,"startColumn":22}}}]}]}]})
}
#[test]
fn unicode_columns_bind_to_same_source_and_every_result_is_validated() {
    let source = "char *s=\"é\"; int x = ;\n".as_bytes();
    let valid = report();
    let rows = codeguard_adapters::parse_clang_stdin_sarif(
        &serde_json::to_vec(&valid).unwrap(),
        source,
        false,
    )
    .unwrap();
    assert_eq!(rows[0]["column_byte"], 23);
    for pointer in [
        "/version",
        "/runs/0/columnKind",
        "/runs/0/tool/driver/version",
        "/runs/0/results/0/ruleIndex",
        "/runs/0/results/0/locations/0/physicalLocation/artifactLocation/uri",
        "/runs/0/results/0/locations/0/physicalLocation/region/startColumn",
    ] {
        let mut bad = valid.clone();
        *bad.pointer_mut(pointer).unwrap() = json!("untrusted");
        assert!(
            codeguard_adapters::parse_clang_stdin_sarif(
                &serde_json::to_vec(&bad).unwrap(),
                source,
                false
            )
            .is_err(),
            "{pointer}"
        );
    }
    let mut second = valid.clone();
    let mut bad = second["runs"][0]["results"][0].clone();
    bad["locations"][0]["physicalLocation"]["region"]["startLine"] = json!(100);
    second["runs"][0]["results"]
        .as_array_mut()
        .unwrap()
        .push(bad);
    assert!(
        codeguard_adapters::parse_clang_stdin_sarif(
            &serde_json::to_vec(&second).unwrap(),
            source,
            false
        )
        .is_err()
    );
    assert!(
        codeguard_adapters::parse_clang_stdin_sarif(
            &serde_json::to_vec(&valid).unwrap(),
            source,
            true
        )
        .is_err()
    );
    let mut empty = valid;
    empty["runs"][0]["results"] = json!([]);
    assert!(
        codeguard_adapters::parse_clang_stdin_sarif(
            &serde_json::to_vec(&empty).unwrap(),
            source,
            false
        )
        .is_err()
    );
    empty["runs"][0]["invocations"][0]["executionSuccessful"] = json!(true);
    empty["runs"][0]["artifacts"] = json!([]);
    assert!(
        codeguard_adapters::parse_clang_stdin_sarif(
            &serde_json::to_vec(&empty).unwrap(),
            source,
            true
        )
        .unwrap()
        .is_empty()
    );
    let bytes = serde_json::to_string(&empty).unwrap().replacen(
        "\"version\":\"2.1.0\"",
        "\"version\":\"1.0\",\"version\":\"2.1.0\"",
        1,
    );
    assert!(codeguard_adapters::parse_clang_stdin_sarif(bytes.as_bytes(), source, true).is_err());
}
