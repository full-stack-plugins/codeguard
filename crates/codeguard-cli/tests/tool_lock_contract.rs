use codeguard_cli::tool_lock::parse_tool_lock_document;
use serde_json::{Value, json};

fn valid() -> Value {
    json!({
        "schema_version":"1.0",
        "lock_id":"local-ruff-0.16.8",
        "tools":[{
            "id":"ruff",
            "version":"0.16.8",
            "binary_sha256":"71f417319c8c4d45562034b44a4ff08e5a1b3dddab5b5c42d5df9800e86a6084",
            "platform":"macos_arm64",
            "adapter":{"id":"python-ruff","version":"0.1.0"},
            "rule_source":{"kind":"native_builtin","id":"ruff-builtin","sha256":"71f417319c8c4d45562034b44a4ff08e5a1b3dddab5b5c42d5df9800e86a6084"},
            "origin":{"kind":"system","ref":"/opt/anaconda3/bin/ruff"}
        }]
    })
}

fn parse(value: &Value) -> Result<codeguard_cli::tool_lock::ToolLock, String> {
    parse_tool_lock_document(&serde_json::to_vec(value).expect("测试 JSON"))
}

#[test]
fn exact_tool_platform_identity_is_readable_but_not_an_approval_claim() {
    let lock = parse(&valid()).expect("有效工具锁");
    let ruff = lock.find("ruff", "macos_arm64").expect("锁定 Ruff");
    assert_eq!(ruff.version, "0.16.8");
    assert_eq!(ruff.origin_kind, "system");
    assert!(lock.find("ruff", "linux_x86_64").is_none());
}

#[test]
fn unknown_major_and_invalid_enums_are_rejected() {
    let mut document = valid();
    document["schema_version"] = json!("2.0");
    assert!(parse(&document).is_err());
    document = valid();
    document["tools"][0]["origin"]["kind"] = json!("download_latest");
    assert!(parse(&document).is_err());
    document = valid();
    document["tools"][0]["rule_source"]["kind"] = json!("formatter");
    assert!(parse(&document).is_err());
}

#[test]
fn truncated_or_uppercase_digests_and_duplicate_identities_are_rejected() {
    let mut document = valid();
    document["tools"][0]["binary_sha256"] = json!("abc");
    assert!(parse(&document).is_err());
    document = valid();
    document["tools"][0]["rule_source"]["sha256"] = json!("A".repeat(64));
    assert!(parse(&document).is_err());
    document = valid();
    let duplicate = document["tools"][0].clone();
    document["tools"]
        .as_array_mut()
        .expect("工具数组")
        .push(duplicate);
    assert!(parse(&document).is_err());
}

#[test]
fn compatible_minor_extensions_are_explicit_and_unknown_authority_fields_fail() {
    let mut document = valid();
    document["schema_version"] = json!("1.1");
    document["extensions"] = json!({"x-display-name":"Ruff"});
    assert!(parse(&document).is_ok());
    document["tools"][0]["unreviewed_rule_override"] = json!("all-off");
    assert!(parse(&document).is_err());
    document = valid();
    document["extensions"] = json!({"rule_override":"all-off"});
    assert!(parse(&document).is_err());
}

#[test]
fn published_schema_declares_strict_version_and_native_identity_fields() {
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/tool-lock.schema.json"))
            .expect("发布的 schema JSON");
    assert_eq!(schema["$id"], "urn:codeguard:schema:tool-lock:1.0");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["$defs"]["tool"]["additionalProperties"], false);
    assert_eq!(
        schema["$defs"]["tool"]["required"]
            .as_array()
            .expect("必需字段")
            .len(),
        7
    );
}

#[test]
fn bundle_tree_identity_requires_a_complete_lowercase_digest() {
    let mut document = valid();
    document["tools"][0]["bundle"] = json!({
        "root":"/opt/tools/ruff",
        "tree_sha256":"a".repeat(64)
    });
    let parsed = parse(&document).expect("bundle lock");
    assert_eq!(
        parsed.tools[0].bundle.as_ref().expect("bundle").root,
        "/opt/tools/ruff"
    );
    document["tools"][0]["bundle"]["tree_sha256"] = json!("short");
    assert!(parse(&document).is_err());
    document["tools"][0]["bundle"]["tree_sha256"] = json!("a".repeat(64));
    document["tools"][0]["bundle"]["unapproved"] = json!(true);
    assert!(parse(&document).is_err());
}
