//! 统一格式命令验收：check/apply/list 三模式与如实披露。
//!
//! 覆盖真实 rustfmt 的完整闭环、不合规拦截、缺工具披露、
//! 未接入语言如实返回、以及 format 与其他类别的分离。

use std::path::PathBuf;
use std::process::Command;

fn ensure_clean_dir(name: &str) -> PathBuf {
    let tmp = std::env::temp_dir().join(format!("{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

fn rustfmt_available() -> bool {
    Command::new("rustfmt").arg("--version").output().is_ok()
}

const UNFORMATTED_RS: &str = "fn main(){\nlet x=1;\n}\n";
const FORMATTED_RS: &str = "fn main() {\n    let x = 1;\n}\n";

/// format list 列出档案并披露接入状态。
#[test]
fn format_list_reports_integration_status() {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["format", "list", "--format=json"])
        .output()
        .expect("无法运行 codeguard format list");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    assert_eq!(report["report_type"], "format_profile_inventory");
    assert_eq!(report["category"], "format");
    assert_eq!(report["language_count"], 57, "档案应覆盖 57 语言");
    assert!(
        report["integrated_count"].as_u64().unwrap_or(0) > 0,
        "应至少有一种语言已接入格式化器"
    );
    // format 类别必须声明与其他类别分离
    assert!(
        report["separation_note"]
            .as_str()
            .is_some_and(|note| note.contains("冒充")),
        "format 类别应声明不冒充注释或规范检查"
    );
}

/// 未接入格式化的语言必须如实返回 not_integrated，不得静默通过。
#[test]
fn unintegrated_language_is_disclosed_not_silently_passed() {
    let root = ensure_clean_dir("cg-format-unintegrated");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["format", "check", "cobol", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard format check");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    assert_eq!(report["status"], "not_integrated");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["exit_code"], 3);
    assert_eq!(output.status.code(), Some(3), "未接入语言应退出 3");
    assert!(
        report["not_integrated_languages"]
            .as_array()
            .is_some_and(|langs| langs.iter().any(|l| l == "cobol")),
        "应列出未接入语言"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// 已接入语言但无源文件时不得声称格式合规。
#[test]
fn no_source_files_does_not_claim_allow() {
    let root = ensure_clean_dir("cg-format-nofiles");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["format", "check", "rust", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard format check");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    assert_ne!(
        report["delivery_decision"], "allow",
        "未观察到源文件不得签发 allow"
    );
    assert_eq!(report["exit_code"], 3);
    let _ = std::fs::remove_dir_all(&root);
}

/// 缺格式化器工具时如实 incomplete，不得伪造通过。
#[test]
fn missing_formatter_is_disclosed() {
    let root = ensure_clean_dir("cg-format-missing-tool");
    std::fs::write(root.join("main.rs"), UNFORMATTED_RS).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "format",
            "check",
            "rust",
            root.to_str().unwrap(),
            "--tool",
            "rustfmt=/nonexistent/rustfmt",
            "--format=json",
        ])
        .output()
        .expect("无法运行 codeguard format check");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    let language = &report["languages"][0];
    assert_eq!(language["status"], "incomplete");
    assert_eq!(language["reason"], "formatter_not_found");
    assert_ne!(report["delivery_decision"], "allow", "缺工具不得签发 allow");
    let _ = std::fs::remove_dir_all(&root);
}

/// 真实 rustfmt：完整格式闭环 check→deny、apply、复检→allow。
#[test]
fn real_rustfmt_check_apply_recheck_loop() {
    if !rustfmt_available() {
        return;
    }
    let root = ensure_clean_dir("cg-format-real-loop");

    // 1. check 不合规源码 → deny
    std::fs::write(root.join("main.rs"), UNFORMATTED_RS).unwrap();
    let first = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["format", "check", "rust", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard format check");
    let first_report: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&first.stdout)).expect("JSON 解析失败");

    assert_eq!(first_report["delivery_decision"], "deny", "不合规应 deny");
    assert_eq!(
        first_report["unformatted_count"], 1,
        "应识别 1 个不合规文件"
    );
    assert_eq!(first.status.code(), Some(1), "不合规应退出 1");
    // check 只读：源码不得被修改
    assert_eq!(
        std::fs::read_to_string(root.join("main.rs")).unwrap(),
        UNFORMATTED_RS,
        "format check 是只读的，不得修改源码"
    );

    // 2. apply 应用格式化
    let applied = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["format", "apply", "rust", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard format apply");
    let applied_report: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&applied.stdout)).expect("JSON 解析失败");
    assert_eq!(applied_report["status"], "reformatted");
    assert_eq!(applied_report["reformatted_count"], 1);
    assert_eq!(applied_report["mutates_sources"], true);
    assert!(
        applied_report["next_action"]
            .as_str()
            .is_some_and(|action| action.contains("复检")),
        "apply 后应提示复检"
    );

    // 3. 复检 → allow
    let second = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["format", "check", "rust", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard format check");
    let second_report: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&second.stdout)).expect("JSON 解析失败");
    assert_eq!(
        second_report["delivery_decision"], "allow",
        "修复后应放行"
    );
    assert_eq!(second_report["unformatted_count"], 0);
    assert_eq!(second.status.code(), Some(0), "合规应退出 0");

    // 格式化器版本与身份应被冻结，防止工具升级后沿用旧结论
    let language = &second_report["languages"][0];
    assert!(
        language["tool_version"].as_str().is_some_and(|v| !v.is_empty()),
        "应记录格式化器版本"
    );
    assert!(
        language["tool_digest"].as_str().is_some_and(|d| !d.is_empty()),
        "应记录格式化器字节身份摘要"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// 已格式化源码初始即为合规。
#[test]
fn already_formatted_source_passes_immediately() {
    if !rustfmt_available() {
        return;
    }
    let root = ensure_clean_dir("cg-format-already-clean");
    std::fs::write(root.join("main.rs"), FORMATTED_RS).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["format", "check", "rust", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard format check");
    let report: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).expect("JSON 解析失败");

    assert_eq!(report["delivery_decision"], "allow", "已格式化源码应放行");
    assert_eq!(report["unformatted_count"], 0);
    let _ = std::fs::remove_dir_all(&root);
}

/// format 类别必须声明独立，不冒充注释或规范检查。
#[test]
fn format_category_is_separated_from_other_checks() {
    let root = ensure_clean_dir("cg-format-separation");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["format", "check", "rust", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard format check");
    let report: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).expect("JSON 解析失败");

    assert_eq!(report["category"], "format");
    assert!(
        report["separation_note"]
            .as_str()
            .is_some_and(|note| note.contains("只判代码风格")),
        "应声明 format 只判代码风格"
    );
    // 不得出现四核心能力类别的签发
    assert!(
        report.get("qualification").is_none(),
        "format 报告不得授予核心能力资格"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// format all 汇总所有已接入语言，未接入者不参与但如实披露。
#[test]
fn format_all_aggregates_integrated_languages_only() {
    let root = ensure_clean_dir("cg-format-all");
    std::fs::write(root.join("main.rs"), FORMATTED_RS).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["format", "check", "all", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard format check");
    let report: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).expect("JSON 解析失败");

    assert_eq!(report["selection"], "all");
    let languages = report["languages"].as_array().expect("应有语言明细");
    assert!(!languages.is_empty(), "format all 应汇总已接入语言");
    for language in languages {
        assert_ne!(
            language["formatter"].as_str().unwrap_or("-"),
            "-",
            "format all 不应包含未接入语言"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// 未知参数应被拒绝并给出非 0 退出码。
#[test]
fn unknown_argument_is_rejected() {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["format", "check", "rust", ".", "--bogus"])
        .output()
        .expect("无法运行 codeguard format check");
    assert_eq!(output.status.code(), Some(2), "参数错误应退出 2");
}