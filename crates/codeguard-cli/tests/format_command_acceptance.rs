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

/// 选中的语言不在注册表内时必须如实拒绝，不得静默通过或伪造报告。
#[test]
fn unregistered_language_is_explicitly_rejected() {
    let root = ensure_clean_dir("cg-format-unregistered");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args([
            "format",
            "check",
            "not-a-registered-language",
            root.to_str().unwrap(),
            "--format=json",
        ])
        .output()
        .expect("无法运行 codeguard format check");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("JSON 解析失败");

    assert_eq!(report["status"], "not_integrated");
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(report["exit_code"], 3);
    assert_eq!(output.status.code(), Some(3), "未注册语言应退出 3");
    // 报告不得凭空出现语言明细（避免伪造「已检查」）
    assert!(
        report["languages"].as_array().map(Vec::is_empty).unwrap_or(true),
        "未注册语言不得产生语言执行明细"
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
/// 多语言混合目录：format all 应分别路由到各语言原生格式化器。
#[test]
fn format_all_routes_per_language_formatters() {
    let root = ensure_clean_dir("cg-format-multi");
    std::fs::write(root.join("main.rs"), FORMATTED_RS).unwrap();
    std::fs::write(root.join("a.tf"), "resource \"null_resource\" \"a\" {\n}\n").unwrap();
    std::fs::write(root.join("q.sql"), "SELECT 1;\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["format", "check", "all", root.to_str().unwrap(), "--format=json"])
        .output()
        .expect("无法运行 codeguard format check all");
    let report: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).expect("JSON 解析失败");

    assert_eq!(report["selection"], "all");
    let languages = report["languages"].as_array().expect("应有语言明细");
    // 每个被路由的语言都应绑定到一个具体格式化器，且不得是未接入项
    for language in languages {
        assert_ne!(
            language["status"], "not_integrated",
            "format all 不应路由到未接入语言: {}",
            language["language"]
        );
        assert!(
            language["formatter"].as_str().is_some_and(|f| !f.is_empty()),
            "每个语言应绑定具体格式化器"
        );
        // 报告明细投影工具身份与扫描结果（argv 从 format list 查，不重复投影）
        assert!(
            language["file_count"].is_number(),
            "{} 应报告扫描文件数",
            language["language"]
        );
        assert!(
            language["tool_version"].is_string() || language["tool_version"].is_null(),
            "{} 应报告工具版本（缺工具时为 null）",
            language["language"]
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// 未接入语言永不随 format all 静默参与。
#[test]
fn format_all_never_includes_not_integrated() {
    let root = ensure_clean_dir("cg-format-all-gap");
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["format", "list", "--format=json"])
        .output()
        .expect("无法运行 codeguard format list");
    let inventory: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).expect("JSON 解析失败");

    let all_rows = inventory["languages"].as_array().expect("档案列表");
    let integrated = all_rows
        .iter()
        .filter(|r| r["status"] == "integrated")
        .count();
    let gap = all_rows
        .iter()
        .filter(|r| r["status"] == "not_integrated")
        .count();
    assert_eq!(integrated + gap, 57, "档案应覆盖 57 语言且无第三种状态");
    // 未接入语言必须保留 formatter=null，杜绝编译期伪造
    for row in all_rows.iter().filter(|r| r["status"] == "not_integrated") {
        assert!(
            row["formatter"].is_null(),
            "{} 未接入时 formatter 必须为 null",
            row["language"]
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// 57 语言必须全部有明确 disposition：要么可执行（有 formatter + argv + 退出码），
/// 要么如实未接入（工具字段为 null + 具体阻塞证据）。不允许第三种状态。
#[test]
fn every_language_has_executable_or_evidenced_disposition() {
    let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["format", "list", "--format=json"])
        .output()
        .expect("无法运行 codeguard format list");
    let report: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).expect("JSON 解析失败");

    let rows = report["languages"].as_array().expect("档案列表");
    assert_eq!(rows.len(), 57, "档案应覆盖 57 语言");

    let mut executable = 0usize;
    let mut disclosed = 0usize;
    for row in rows {
        let language = row["language"].as_str().unwrap_or("?");
        let notes = row["notes"].as_str().unwrap_or("");
        assert!(notes.len() >= 20, "{language} 应有实质 notes 依据");

        match row["status"].as_str() {
            Some("integrated") => {
                executable += 1;
                assert!(
                    row["formatter"].as_str().is_some_and(|f| !f.is_empty()),
                    "{language} integrated 但缺 formatter"
                );
                assert!(
                    row["check_argv"].as_array().is_some_and(|a| !a.is_empty()),
                    "{language} integrated 但缺 check_argv"
                );
                assert!(
                    row["apply_argv"].as_array().is_some_and(|a| !a.is_empty()),
                    "{language} integrated 但缺 apply_argv"
                );
                assert!(
                    row["unformatted_exit_codes"]
                        .as_array()
                        .is_some_and(|c| !c.is_empty()),
                    "{language} integrated 但缺 unformatted_exit_codes"
                );
                assert!(
                    !row["extensions"].as_array().unwrap_or(&vec![]).is_empty(),
                    "{language} integrated 但缺 extensions"
                );
            }
            Some("not_integrated") => {
                disclosed += 1;
                // 未接入必须工具字段为 null，杜绝编译期伪造
                assert!(
                    row["formatter"].is_null() && row["tool_key"].is_null(),
                    "{language} 未接入时 formatter/tool_key 必须为 null"
                );
                assert!(
                    row["check_argv"].is_null() && row["apply_argv"].is_null(),
                    "{language} 未接入时 check_argv/apply_argv 必须为 null"
                );
            }
            other => panic!("{language} 出现非法 status: {other:?}（只允许 integrated / not_integrated）"),
        }
    }
    assert_eq!(
        executable + disclosed,
        57,
        "每种语言必须有且仅有一种 disposition"
    );
    assert!(
        executable >= 50,
        "可执行门禁应覆盖至少 50 种语言，当前 {executable}"
    );
}

/// 交付口径必须自洽：54 种 stable 全部具备可执行门禁，3 种 planned 正式排除。
///
/// 这是对「剩余 50 种全部接入」的可审计回答：planned 语言从未进入 stable 交付口径，
/// 路由条目（ansible→yaml）是可执行的真实门禁而非缺口声明。
#[test]
fn format_gate_scope_accounting_is_complete_and_auditable() {
    let scope: serde_json::Value = serde_json::from_str(include_str!(
        "../../../rulepacks/format_gate_scope.json"
    ))
    .expect("format_gate_scope.json 应可解析");

    let inventory: serde_json::Value = serde_json::from_slice(
        &Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["format", "list", "--format=json"])
            .output()
            .expect("无法运行 codeguard format list")
            .stdout,
    )
    .expect("JSON 解析失败");

    let total = scope["registry_total"].as_u64().expect("registry_total");
    let stable = scope["registry_stable"].as_u64().expect("registry_stable");
    let planned = scope["registry_planned"].as_u64().expect("registry_planned");
    let integrated = scope["integrated_count"].as_u64().expect("integrated_count");
    let excluded = scope["planned_excluded"].as_array().expect("planned 排除列表");
    let gaps = scope["explicit_gaps"].as_array().expect("显式归口列表");

    // 注册表口径自洽
    assert_eq!(stable + planned, total, "stable + planned 必须等于总数");
    // 全部 stable 必须有可执行门禁，不允许留缺口
    let integrated_langs: Vec<&str> = scope["integrated_languages"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    let excluded_langs: Vec<&str> = excluded
        .iter()
        .filter_map(|e| e["language"].as_str())
        .collect();
    for language in &integrated_langs {
        assert!(
            !excluded_langs.contains(language),
            "{language} 同时出现在已接入与排除集合中"
        );
    }
    // stable 全部必须已接入（planned 可部分排除，但排除项必须真为 planned）
    assert!(
        integrated >= stable,
        "全部 {stable} 种 stable 语言必须具备可执行格式门禁，当前仅 {integrated}"
    );
    for language in &excluded_langs {
        assert!(
            !integrated_langs.contains(language),
            "{language} 被排除但已接入集合中也有它"
        );
    }
    // 三类恰好穷尽：已接入 + planned 排除 + 显式缺口
    assert_eq!(
        integrated as usize + excluded.len() + gaps.len(),
        total as usize,
        "已接入 + planned 排除 + 显式缺口必须恰好穷尽 57，不允许静默遗漏"
    );
    assert!(gaps.is_empty(), "stable 语言不应留有显式缺口");

    // planned 排除项必须真为 planned 且引用仓库内可验证证据
    for entry in excluded {
        let language = entry["language"].as_str().unwrap();
        assert_eq!(
            entry["registry_status"], "planned",
            "{language} 被排除但注册表状态非 planned"
        );
        assert!(
            !entry["evidence"].as_array().unwrap_or(&vec![]).is_empty(),
            "{language} 的 planned 排除必须引用仓库内可验证证据"
        );
    }

    // 路由条目必须指向已接入的目标通道，且带可执行 argv（不是缺口声明）
    for entry in scope["routed_entries"].as_array().unwrap_or(&vec![]) {
        let language = entry["language"].as_str().unwrap();
        let target = entry["routed_to"].as_str().unwrap();
        assert_ne!(language, target, "{language} 不应路由到自身");
        assert!(
            scope["integrated_languages"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v.as_str() == Some(target)),
            "{language} 的路由目标 {target} 必须本身是已接入通道"
        );
    }

    // 口径声明的已接入集合必须与真实档案逐字同序（防止口径与实现漂移）
    let actual: Vec<&str> = inventory["languages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["status"] == "integrated")
        .map(|r| r["language"].as_str().unwrap())
        .collect();
    let declared: Vec<&str> = scope["integrated_languages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(
        actual, declared,
        "口径声明的已接入集合必须与 format list 实际输出逐字一致"
    );
}

/// 工具清单必须与格式化档案一致：每个已接入语言都能查到安装方式。
///
/// 门禁不安装工具（见 rulepacks/format_toolchain.json 的 policy 字段），
/// 因此「这个语言的工具从哪来」必须有机器可读的答案；
/// 缺答案意味着真实环境无法准备，门禁会永远停在 formatter_not_found。
#[test]
fn every_integrated_language_has_declared_tool_install() {
    let profiles: serde_json::Value = serde_json::from_str(include_str!(
        "../../../rulepacks/format_profiles.json"
    ))
    .expect("format_profiles.json 应可解析");
    let toolchain: serde_json::Value = serde_json::from_str(include_str!(
        "../../../rulepacks/format_toolchain.json"
    ))
    .expect("format_toolchain.json 应可解析");

    // 门禁不安装工具：这是安全与可复现性的前提，必须留在文件里
    let policy = toolchain["policy"].as_str().unwrap_or("");
    assert!(
        policy.contains("不安装"),
        "工具清单必须显式声明「codeguard 不安装任何工具」——这是安全与可复现性前提"
    );

    let mut declared: Vec<&str> = Vec::new();
    for tool in toolchain["tools"].as_array().expect("tools 列表") {
        for language in tool["languages"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
            if let Some(name) = language.as_str() {
                declared.push(name);
            }
        }
    }

    for row in profiles["languages"].as_array().unwrap() {
        if row["status"] != "integrated" {
            continue;
        }
        let language = row["language"].as_str().unwrap();
        assert!(
            declared.contains(&language),
            "{language} 已接入格式化门禁，但工具清单里没有它的安装方式——\
真实环境无法准备，该语言的门禁会永远停在 formatter_not_found"
        );
    }

    // 清单自身必须完整：每个工具都要有安装方式（归口条目除外）
    for tool in toolchain["tools"].as_array().unwrap() {
        let kind = tool["install_kind"].as_str().unwrap_or("");
        if kind == "routed" {
            continue;
        }
        assert!(
            tool["install"].as_str().is_some_and(|i| !i.trim().is_empty()),
            "工具 {} 缺少安装方式声明",
            tool["formatter"].as_str().unwrap_or("?")
        );
    }
}
