use crate::{CheckerConfiguration, strict_json};
/// 只读识别package.json中的明确npm audit脚本；不执行脚本或推断外部CI配置。
/// 参数为原清单字节、构建根及来源引用；调用方仍须核对当前字节身份。
pub fn inspect_npm_audit_config(bytes: &[u8], root: &str, source: &str) -> CheckerConfiguration {
    let mut result=CheckerConfiguration{build_root:root.into(),checker_id:"node.npm.audit".into(),category:"cve".into(),configuration:"unknown".into(),configuration_ref:source.into(),reason:"npm_audit_declaration_unknown".into(),next_action:"核对项目原npm审计调用、工具版本、锁文件、配置及漏洞数据时效；不执行清单中的任意脚本、不用缺工具解释源码违规".into()};
    if bytes.is_empty() || bytes.len() > 256 * 1024 {
        result.reason = "npm_manifest_invalid".into();
        return result;
    }
    let Ok(value) = strict_json::parse(bytes) else {
        result.reason = "npm_manifest_invalid".into();
        return result;
    };
    if !value.is_object() {
        result.reason = "npm_manifest_invalid".into();
        return result;
    }
    let Some(scripts) = value.get("scripts") else {
        result.configuration = "missing".into();
        result.reason = "npm_audit_script_not_observed".into();
        return result;
    };
    let Some(scripts) = scripts.as_object() else {
        result.configuration = "invalid".into();
        result.reason = "npm_scripts_invalid".into();
        return result;
    };
    let mut recognized = false;
    let mut unknown = false;
    for script in scripts.values() {
        let Some(script) = script.as_str() else {
            result.configuration = "invalid".into();
            result.reason = "npm_scripts_invalid".into();
            return result;
        };
        if matches!(
            script.trim(),
            "npm audit"
                | "npm audit --json"
                | "npm audit --package-lock-only"
                | "npm audit --json --package-lock-only"
        ) {
            recognized = true;
        } else if script.contains("npm audit") {
            unknown = true;
        }
    }
    if unknown {
        result.reason = "npm_audit_custom_invocation_requires_review".into();
    } else if recognized {
        result.configuration = "configured".into();
        result.reason = "npm_audit_runtime_context_unverified".into();
    } else {
        result.configuration = "missing".into();
        result.reason = "npm_audit_script_not_observed".into();
    }
    result
}
