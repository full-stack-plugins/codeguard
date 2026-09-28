use crate::check_budget::{DEFAULT_CHECK_TIMEOUT_MS, parse_check_timeout};
use std::{collections::BTreeMap, path::PathBuf};
/// npm局部CVE命令参数；不自动安装工具或继承宿主凭据。
pub(crate) struct NpmAuditArguments {
    pub(crate) root: PathBuf,
    pub(crate) options: BTreeMap<String, String>,
    pub(crate) json: bool,
    pub(crate) timeout_ms: u64,
}
impl NpmAuditArguments {
    pub(crate) fn parse(args: &[String]) -> Result<Self, &'static str> {
        if args.first().map(String::as_str) != Some("typescript") {
            return Err("cve当前需要typescript语言和明确npm上下文");
        }
        let mut result = Self {
            root: ".".into(),
            options: BTreeMap::new(),
            json: false,
            timeout_ms: DEFAULT_CHECK_TIMEOUT_MS,
        };
        let mut has_root = false;
        let mut i = 1;
        while i < args.len() {
            let key = &args[i];
            if key.starts_with('-') {
                if !matches!(
                    key.as_str(),
                    "--node-tool"
                        | "--npm-entry"
                        | "--npm-version"
                        | "--userconfig"
                        | "--globalconfig"
                        | "--workspace"
                        | "--registry"
                        | "--format"
                        | "--timeout"
                ) || result.options.contains_key(key)
                {
                    return Err("npm审计参数未知或重复");
                }
                i += 1;
                let value = args
                    .get(i)
                    .filter(|v| !v.starts_with('-') && !v.chars().any(char::is_control))
                    .ok_or("npm审计参数缺少安全值")?;
                match key.as_str() {
                    "--format" => {
                        if !matches!(value.as_str(), "human" | "json") {
                            return Err("npm审计格式未知");
                        }
                        result.json = value == "json";
                    }
                    "--timeout" => {
                        result.timeout_ms =
                            parse_check_timeout(value).map_err(|_| "npm审计超时预算无效")?
                    }
                    "--npm-version" | "--registry" => {}
                    _ => {
                        if !PathBuf::from(value).is_absolute() {
                            return Err("npm工具与配置需要绝对路径");
                        }
                    }
                }
                result.options.insert(key.clone(), value.clone());
            } else {
                if has_root || key.chars().any(char::is_control) {
                    return Err("npm审计需要唯一安全项目目录");
                }
                result.root = key.into();
                has_root = true;
            }
            i += 1;
        }
        Ok(result)
    }
}
