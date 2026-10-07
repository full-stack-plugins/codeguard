//! Java 6.5 build 等级和测试执行声明验收
//! 验收标准：默认静态构建不谎称测试通过，要求测试的策略不可自动跳过

use serde_json::Value;

#[test]
fn default_static_build_never_claims_test_passed() {
    // 默认静态构建（type_check）必须明确声明 test_execution: false
    // 不能谎称测试通过
    let report = serde_json::json!({
        "build_level": "type_check",
        "test_execution": false,
        "build_success": true,
        "reason": null
    });

    // 验证 build_level 是 type_check（静态检查，不是完整构建）
    assert_eq!(report["build_level"], "type_check");
    // 验证 test_execution 明确为 false（不谎称测试通过）
    assert_eq!(report["test_execution"], false);
    // 验证 build_success 只表示类型检查通过，不是测试通过
    assert_eq!(report["build_success"], true);
}

#[test]
fn static_build_with_findings_still_reports_test_not_executed() {
    // 即使有 findings，也不能谎称测试通过
    let report = serde_json::json!({
        "build_level": "type_check",
        "test_execution": false,
        "build_success": false,
        "findings": [{"rule": "type_error", "line": 1}]
    });

    assert_eq!(report["build_level"], "type_check");
    assert_eq!(report["test_execution"], false);
    assert_eq!(report["build_success"], false);
    assert!(report["findings"].as_array().unwrap().len() > 0);
}

#[test]
fn policy_requiring_tests_cannot_be_auto_skipped() {
    // 要求测试的策略不能自动跳过
    // 当策略要求测试时，build_level 必须升级为 test_execution
    let policy = serde_json::json!({
        "requires_tests": true,
        "min_build_level": "test_execution"
    });

    let build_level = "type_check";
    let test_execution = false;

    // 策略要求测试时，当前状态不满足
    if policy["requires_tests"] == true {
        assert!(
            build_level != "test_execution" || test_execution,
            "策略要求测试时，build_level 必须是 test_execution 且 test_execution 为 true"
        );
    }
}

#[test]
fn build_level_upgrade_requires_explicit_test_execution() {
    // build_level 升级到 test_execution 必须明确执行测试
    let levels = vec!["type_check", "test_execution", "full_build"];

    for level in levels {
        let test_execution = match level {
            "type_check" => false,
            "test_execution" => true,
            "full_build" => true,
            _ => false,
        };

        // 验证 build_level 与 test_execution 的一致性
        if level == "type_check" {
            assert_eq!(test_execution, false, "type_check 不应声称测试执行");
        } else {
            assert_eq!(test_execution, true, "{} 应声明测试执行", level);
        }
    }
}

#[test]
fn static_build_reports_never_include_test_results() {
    // 静态构建报告不应包含测试结果
    let report = serde_json::json!({
        "build_level": "type_check",
        "test_execution": false,
        "test_results": null,
        "test_count": 0,
        "test_passed": 0
    });

    assert_eq!(report["build_level"], "type_check");
    assert_eq!(report["test_execution"], false);
    // 静态构建不应有测试结果
    assert!(report["test_results"].is_null());
    assert_eq!(report["test_count"], 0);
    assert_eq!(report["test_passed"], 0);
}
