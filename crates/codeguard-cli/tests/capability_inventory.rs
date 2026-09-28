use codeguard_adapters::validate_capability_inventory;
use serde_json::{Value, json};
use std::process::Command;

fn inventory() -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["capabilities", "--format", "json"])
        .output()
        .expect("能力命令应可执行");
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).expect("能力 JSON")
}

#[test]
fn all_legacy_languages_and_cells_are_explicit() {
    validate_capability_inventory(&inventory()).expect("当前能力矩阵完整");
    let document = inventory();
    assert_eq!(document["schema_version"], "0.2.0");
    let categories = &document["languages"][0]["platforms"]["macos_arm64"];
    assert_eq!(categories.as_object().unwrap().len(), 6);
    assert_eq!(categories["dependencies"]["status"], "gap");
}

#[test]
fn missing_platform_category_is_rejected() {
    let mut changed = inventory();
    changed["languages"][0]["platforms"]["macos_arm64"]
        .as_object_mut()
        .expect("类别对象")
        .remove("lint");
    assert!(validate_capability_inventory(&changed).is_err());
}

#[test]
fn implemented_without_evidence_is_rejected() {
    let mut changed = inventory();
    changed["languages"][0]["platforms"]["macos_arm64"]["lint"]["status"] = json!("implemented");
    assert!(validate_capability_inventory(&changed).is_err());
}

#[test]
fn formatter_only_cannot_claim_lint() {
    let mut changed = inventory();
    let julia = changed["languages"]
        .as_array_mut()
        .expect("语言数组")
        .iter_mut()
        .find(|row| row["language"] == "julia")
        .expect("Julia 登记");
    julia["platforms"]["macos_arm64"]["lint"] = json!({
        "status":"implemented",
        "reason":"formatter",
        "evidence_ref":"fake",
        "tool_kind":"formatter"
    });
    assert!(validate_capability_inventory(&changed).is_err());
}
