use codeguard_cli::check_request::parse_check_request;
use serde_json::{Value, json};

fn valid() -> Value {
    json!({
        "schema_version":"1.0",
        "report_type":"check_request",
        "request_id":"run-001",
        "command":"lint",
        "selection":{"language":"python"},
        "root":"/workspace/example",
        "content_source":{"kind":"working_tree"},
        "options":{"format":"json","offline":false,"timeout_ms":30000,"jobs":2}
    })
}

fn parse(value: &Value) -> Result<codeguard_cli::check_request::CheckRequest, String> {
    parse_check_request(&serde_json::to_vec(value).expect("测试 JSON"))
}

#[test]
fn valid_request_keeps_selection_separate_from_quality_result() {
    let request = parse(&valid()).expect("有效请求");
    assert_eq!(request.command, "lint");
    assert_eq!(request.language, "python");
    assert_eq!(request.timeout_ms, 30000);
}

#[test]
fn unknown_major_command_content_kind_and_format_are_rejected() {
    let mut document = valid();
    document["schema_version"] = json!("2.0");
    assert!(parse(&document).is_err());
    document = valid();
    document["command"] = json!("skip");
    assert!(parse(&document).is_err());
    document = valid();
    document["content_source"]["kind"] = json!("whatever_is_cached");
    assert!(parse(&document).is_err());
    document = valid();
    document["options"]["format"] = json!("unknown");
    assert!(parse(&document).is_err());
}

#[test]
fn explicit_content_identity_and_operational_bounds_are_required() {
    let mut document = valid();
    document["content_source"]["kind"] = json!("ci");
    assert!(parse(&document).is_err());
    document["content_source"]["identity"] = json!("sha256:immutable-content");
    assert!(parse(&document).is_ok());
    document = valid();
    document["options"]["timeout_ms"] = json!(0);
    assert!(parse(&document).is_err());
    document = valid();
    document["options"]["jobs"] = json!(65);
    assert!(parse(&document).is_err());
}

#[test]
fn extensions_do_not_accept_unreviewed_authority_fields() {
    let mut document = valid();
    document["extensions"] = json!({"x-trace-label":"local"});
    document["schema_version"] = json!("1.1");
    assert!(parse(&document).is_ok());
    document["options"]["skip_security"] = json!(true);
    assert!(parse(&document).is_err());
    document = valid();
    document["extensions"] = json!({"override_policy":"allow"});
    assert!(parse(&document).is_err());
}

#[test]
fn published_request_schema_is_parseable_and_strict() {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/check-request.schema.json"))
            .expect("请求 schema JSON");
    assert_eq!(schema["$id"], "urn:codeguard:schema:check-request:1.0");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["options"]["additionalProperties"],
        false
    );
}

#[test]
fn duplicate_nested_options_cannot_hide_an_earlier_value() {
    let raw = serde_json::to_string(&valid()).unwrap();
    let duplicated = raw.replacen("\"jobs\":2", "\"jobs\":64,\"jobs\":2", 1);
    assert_ne!(duplicated, raw);
    assert!(parse_check_request(duplicated.as_bytes()).is_err());
}
