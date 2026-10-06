//! 从本轮已完成的原生文件结果投影文档规则选择；不扩大生效规则或批准覆盖。
use crate::python_lint_scan::PythonLintScanResult;
use serde_json::{Value, json};

/// 从本轮最终扫描结果生成文档配置观察，保留局部身份和未解析忽略范围。
pub(crate) fn observe(scan: &PythonLintScanResult) -> Value {
    let mut selected = 0;
    let mut unselected = 0;
    let mut unavailable_count = 0;
    let files: Vec<_> = scan.files.iter().map(|file| {
        let settings = file.rule_settings.as_ref().filter(|_| file.completion);
        let rules = settings.map(|s| s.globally_enabled_documentation_rules());
        let status = match &rules {
            Some(rules) if !rules.is_empty() => { selected += 1; "selected" },
            Some(_) => { unselected += 1; "not_selected" },
            None => { unavailable_count += 1; "unavailable" },
        };
        // 未完成结果不保留旧配置/工具/源码身份，避免把历史设置作为当前选择证据。
        json!({"path":file.path,"status":status,
            "globally_enabled_documentation_rules":rules,
            "per_file_ignores_present":settings.map(|s|s.per_file_ignores_present),
            "settings_sha256":settings.map(|s|&s.settings_sha256),
            "config_ref":if settings.is_some(){file.config_ref.as_ref()}else{None},
            "config_sha256":if settings.is_some(){file.config_sha256.as_ref()}else{None},
            "source_sha256":if settings.is_some(){file.source_sha256.as_ref()}else{None},
            "tool_sha256":if settings.is_some(){file.tool_sha256.as_ref()}else{None},
            "reason":if settings.is_some(){None}else{Some(file.reason.as_deref().unwrap_or("native_settings_unavailable"))}})
    }).collect();
    let status = if files.is_empty() {
        "no_python_sources"
    } else if unavailable_count == 0 {
        "observed"
    } else if selected + unselected > 0 {
        "partial"
    } else {
        "unavailable"
    };
    json!({"status":status,"scope":"global_rule_selection_only","coverage_proven":false,
        "source_file_count":files.len(),"selected_file_count":selected,"unselected_file_count":unselected,
        "unavailable_file_count":unavailable_count,"files":files,
        "reason":if status=="no_python_sources"{Some("no_discovered_python_sources")}else{None},
        "next_action":if unavailable_count>0||selected+unselected==0{"恢复原生检查和配置，获取同轮文档规则设置；不凭零诊断判定合规。"}
            else if unselected>0{"核对未选择文档规则的文件与项目文档规范，明确配置所需文档规则；不会自动修改配置或开启preview。"}
            else{"核对已观察文档规则、逐文件忽略及源码抑制；完整详细说明和语义覆盖仍须验收。"}})
}

/// 为根目录或注册表不可用返回显式未知配置，不把零文件解释为已检查。
pub(crate) fn unavailable(reason: &str) -> Value {
    json!({"status":"unavailable","scope":"global_rule_selection_only","coverage_proven":false,
        "source_file_count":0,"selected_file_count":0,"unselected_file_count":0,"unavailable_file_count":0,
        "files":[],"reason":reason,"next_action":"恢复项目根与原生检查环境，取得同轮文档规则设置。"})
}
