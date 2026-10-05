use std::path::PathBuf;
/// Rust独立lint参数；解析成功前不观察项目或启动原生工具。
pub(crate) struct RustLintArguments {
    pub root: PathBuf,
    pub tool: Option<PathBuf>,
    pub json: bool,
    pub timeout: Option<u64>,
}
impl RustLintArguments {
    /// 解析路径、唯一工具/格式/预算；返回有效参数或使用错误。
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let (mut root, mut tool, mut format, mut timeout) = (None, None, None, None);
        let mut i = 0;
        while i < args.len() {
            let word = args[i].as_str();
            match word {
                "--cargo-tool" | "--format" | "--timeout" => {
                    i += 1;
                    let value = args.get(i).ok_or_else(|| format!("{word} 缺少值"))?;
                    match word {
                        "--cargo-tool" => {
                            let path = PathBuf::from(value);
                            if !path.is_absolute() || tool.replace(path).is_some() {
                                return Err("--cargo-tool 必须为唯一绝对路径".into());
                            }
                        }
                        "--format" => {
                            if format.replace(parse_format(value)?).is_some() {
                                return Err("--format 不能重复".into());
                            }
                        }
                        _ => {
                            if timeout
                                .replace(crate::check_budget::parse_check_timeout(value)?)
                                .is_some()
                            {
                                return Err("--timeout 不能重复".into());
                            }
                        }
                    }
                }
                value if value.starts_with("--format=") => {
                    if format.replace(parse_format(&value[9..])?).is_some() {
                        return Err("--format 不能重复".into());
                    }
                }
                value if !value.starts_with('-') && root.is_none() => {
                    root = Some(PathBuf::from(value))
                }
                _ => return Err("lint rust 参数无效".into()),
            }
            i += 1;
        }
        Ok(Self {
            root: root.unwrap_or_else(|| PathBuf::from(".")),
            tool,
            json: format.unwrap_or(false),
            timeout,
        })
    }
}
fn parse_format(value: &str) -> Result<bool, String> {
    match value {
        "json" => Ok(true),
        "human" => Ok(false),
        _ => Err("--format 仅支持 human|json".into()),
    }
}
