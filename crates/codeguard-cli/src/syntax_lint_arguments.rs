//! 统一单文件 lint 参数及明确的原生上下文；来源：OpenSpec unified-cli-contract。
use std::{collections::BTreeSet, path::PathBuf};

/// 单文件候选或明确Clang上下文请求；不接受自由argv或隐式安装。
pub(crate) struct SyntaxLintArguments {
    pub(crate) language: String,
    pub(crate) source: PathBuf,
    pub(crate) workspace: Option<PathBuf>,
    pub(crate) clang_tool: Option<PathBuf>,
    pub(crate) standard: Option<String>,
    pub(crate) json: bool,
    pub(crate) timeout_ms: u64,
}
impl SyntaxLintArguments {
    /// 解析规范语言及唯一文件；返回错参原因，所有参数在检查与持久化前核验。
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        let language = args.first().ok_or("lint 需要注册表语言及文件")?;
        if !codeguard_adapters::legacy_registry()?
            .languages
            .iter()
            .any(|row| row.id == *language)
        {
            return Err("lint 需要注册表规范语言 ID".into());
        }
        let mut result = Self {
            language: language.clone(),
            source: PathBuf::new(),
            workspace: None,
            clang_tool: None,
            standard: None,
            json: false,
            timeout_ms: crate::check_budget::DEFAULT_CHECK_TIMEOUT_MS,
        };
        let mut seen = BTreeSet::new();
        let mut index = 1;
        while index < args.len() {
            let current = &args[index];
            if current.starts_with('-') {
                let (key, inline) = current
                    .split_once('=')
                    .map_or((current.as_str(), None), |(k, v)| (k, Some(v)));
                if !matches!(
                    key,
                    "--workspace" | "--format" | "--timeout" | "--clang-tool" | "--standard"
                ) || !seen.insert(key)
                {
                    return Err("lint 参数未知或重复".into());
                }
                let value = match inline {
                    Some(v) => v,
                    None => {
                        index += 1;
                        args.get(index)
                            .map(String::as_str)
                            .ok_or("lint 参数缺少值")?
                    }
                };
                if value.is_empty() || value.starts_with('-') || value.chars().any(char::is_control)
                {
                    return Err("lint 参数值无效".into());
                }
                match key {
                    "--format" if matches!(value, "json" | "human") => {
                        result.json = value == "json"
                    }
                    "--timeout" => {
                        result.timeout_ms = crate::check_budget::parse_check_timeout(value)?
                    }
                    "--workspace" if PathBuf::from(value).is_absolute() => {
                        result.workspace = Some(PathBuf::from(value))
                    }
                    "--clang-tool"
                        if matches!(language.as_str(), "c" | "cpp")
                            && PathBuf::from(value).is_absolute() =>
                    {
                        result.clang_tool = Some(PathBuf::from(value))
                    }
                    "--standard"
                        if (language == "c" && value == "c11")
                            || (language == "cpp" && value == "c++17") =>
                    {
                        result.standard = Some(value.to_owned())
                    }
                    _ => return Err("lint 格式、上下文或工具路径无效".into()),
                }
            } else {
                if !result.source.as_os_str().is_empty() || current.chars().any(char::is_control) {
                    return Err("lint 需要唯一安全文件路径".into());
                }
                result.source = PathBuf::from(current);
            }
            index += 1;
        }
        if result.source.as_os_str().is_empty() {
            return Err("lint 需要显式源码文件".into());
        }
        if result.clang_tool.is_some() != result.standard.is_some() {
            return Err("Clang工具与标准必须同时明确提供".into());
        }
        Ok(result)
    }
}
