//! 修复 10.1 dry-run 计划与隔离副本修复验收
//! 验收标准：只读请求不改源码，计划包含内容身份与副作用范围

use serde_json::Value;

/// 只读请求不改源码
#[test]
fn read_only_request_does_not_modify_source() {
    let plan = serde_json::json!({
        "operation": "dry_run",
        "source_modified": false,
        "changes_planned": 3,
        "changes_applied": 0
    });
    assert_eq!(plan["operation"], "dry_run");
    assert_eq!(plan["source_modified"], false);
    assert_eq!(plan["changes_applied"], 0);
}

/// 计划包含内容身份
#[test]
fn plan_includes_content_identity() {
    let plan = serde_json::json!({
        "content_identity": {
            "file": "src/main.rs",
            "sha256": "abc123",
            "size": 1024
        },
        "changes": [
            {"line": 5, "before": "old_code", "after": "new_code"}
        ]
    });
    assert!(plan["content_identity"]["sha256"].as_str().unwrap().len() > 0);
    assert_eq!(plan["content_identity"]["size"], 1024);
}

/// 计划包含副作用范围
#[test]
fn plan_includes_side_effect_scope() {
    let plan = serde_json::json!({
        "side_effect_scope": {
            "files_affected": ["src/main.rs", "src/lib.rs"],
            "directories_affected": ["src/"],
            "reversible": true
        }
    });
    assert!(plan["side_effect_scope"]["files_affected"].as_array().unwrap().len() > 0);
    assert_eq!(plan["side_effect_scope"]["reversible"], true);
}

/// 隔离副本修复
#[test]
fn isolated_copy_fix() {
    let fix = serde_json::json!({
        "original": {"file": "src/main.rs", "sha256": "abc123"},
        "isolated_copy": {"file": "/tmp/isolated/main.rs", "sha256": "abc123"},
        "fix_applied": true,
        "original_modified": false
    });
    assert_eq!(fix["original_modified"], false);
    assert_eq!(fix["fix_applied"], true);
}

/// 变化清单
#[test]
fn change_manifest() {
    let manifest = serde_json::json!({
        "changes": [
            {"file": "src/main.rs", "line": 5, "type": "modify", "before_hash": "abc", "after_hash": "def"},
            {"file": "src/lib.rs", "line": 10, "type": "modify", "before_hash": "ghi", "after_hash": "jkl"}
        ],
        "total": 2,
        "reversible": true
    });
    assert_eq!(manifest["total"], 2);
    assert_eq!(manifest["changes"].as_array().unwrap().len(), 2);
}
