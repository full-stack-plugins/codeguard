//! ESLint 局部命令参数；不自动安装工具或迁移配置。
use crate::check_budget::{DEFAULT_CHECK_TIMEOUT_MS, parse_check_timeout};
use std::{collections::BTreeSet, path::PathBuf};
/// 局部原生检查请求；完整项目范围仍由后续调度器负责。
#[derive(Clone)]
pub(crate) struct EslintLintArguments {
    pub(crate) source: PathBuf,
    pub(crate) node: Option<PathBuf>,
    pub(crate) entry: Option<PathBuf>,
    pub(crate) config: Option<PathBuf>,
    pub(crate) config_map: Option<PathBuf>,
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) workspace: Option<PathBuf>,
    pub(crate) version: Option<String>,
    pub(crate) json: bool,
    pub(crate) timeout_ms: u64,
}
impl EslintLintArguments {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        let mut result = Self {
            source: PathBuf::from("."),
            node: None,
            entry: None,
            config: None,
            config_map: None,
            cwd: None,
            workspace: None,
            version: None,
            json: false,
            timeout_ms: DEFAULT_CHECK_TIMEOUT_MS,
        };
        let mut seen = BTreeSet::new();
        let mut source = false;
        let mut index = 0;
        while index < args.len() {
            let current = &args[index];
            if current.starts_with('-') {
                if !matches!(
                    current.as_str(),
                    "--node-tool"
                        | "--eslint-entry"
                        | "--config"
                        | "--config-map"
                        | "--cwd"
                        | "--workspace"
                        | "--eslint-version"
                        | "--format"
                        | "--timeout"
                ) || !seen.insert(current.clone())
                {
                    return Err("ESLint 参数未知或重复".into());
                }
                index += 1;
                let value = args
                    .get(index)
                    .filter(|value| !value.starts_with('-'))
                    .ok_or("ESLint 参数缺少值")?;
                match current.as_str() {
                    "--format" => {
                        if !matches!(value.as_str(), "human" | "json") {
                            return Err("ESLint 格式未知".into());
                        }
                        result.json = value == "json";
                    }
                    "--timeout" => result.timeout_ms = parse_check_timeout(value)?,
                    "--eslint-version" => {
                        if !codeguard_adapters::eslint_report_version_matches(value, value) {
                            return Err("需要具体稳定 ESLint 10 版本".into());
                        }
                        result.version = Some(value.clone());
                    }
                    _ => {
                        let path = PathBuf::from(value);
                        if !path.is_absolute() || value.chars().any(char::is_control) {
                            return Err("工具及原配置需要安全绝对路径".into());
                        }
                        match current.as_str() {
                            "--node-tool" => result.node = Some(path),
                            "--eslint-entry" => result.entry = Some(path),
                            "--config-map" => result.config_map = Some(path),
                            "--cwd" => result.cwd = Some(path),
                            "--workspace" => result.workspace = Some(path),
                            _ => result.config = Some(path),
                        }
                    }
                }
            } else {
                if source || current.chars().any(char::is_control) {
                    return Err("需要唯一安全源码路径".into());
                }
                source = true;
                result.source = PathBuf::from(current);
            }
            index += 1;
        }
        Ok(result)
    }
}
