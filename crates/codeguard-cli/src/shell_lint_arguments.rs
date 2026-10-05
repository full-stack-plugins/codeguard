use std::path::PathBuf;
/// Shell单文件检查参数；先完成全部校验，随后才读取配置或运行工具。
pub(crate) struct ShellLintArguments {
    pub file: PathBuf,
    pub tool: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub dialect: String,
    pub json: bool,
    pub timeout: Option<u64>,
}
impl ShellLintArguments {
    /// 解析唯一文件、方言、绝对工具/配置、格式与预算；返回使用错误不产生进程副作用。
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let (mut file, mut tool, mut config, mut dialect, mut format, mut timeout) =
            (None, None, None, None, None, None);
        let mut i = 0;
        while i < args.len() {
            let word = args[i].as_str();
            match word {
                "--shellcheck-tool"
                | "--shellcheck-config"
                | "--dialect"
                | "--timeout"
                | "--format" => {
                    i += 1;
                    let value = args.get(i).ok_or_else(|| format!("{word}缺少值"))?;
                    match word {
                        "--shellcheck-tool" | "--shellcheck-config" => {
                            let path = PathBuf::from(value);
                            let slot = if word == "--shellcheck-tool" {
                                &mut tool
                            } else {
                                &mut config
                            };
                            if !path.is_absolute() || slot.replace(path).is_some() {
                                return Err(format!("{word}必须为唯一绝对路径"));
                            }
                        }
                        "--dialect" => {
                            if value.is_empty()
                                || value.len() > 16
                                || !value.bytes().all(|b| b.is_ascii_lowercase())
                                || dialect.replace(value.clone()).is_some()
                            {
                                return Err("方言无效或重复".into());
                            }
                        }
                        "--timeout" => {
                            if timeout
                                .replace(crate::check_budget::parse_check_timeout(value)?)
                                .is_some()
                            {
                                return Err("超时重复".into());
                            }
                        }
                        _ => {
                            if format.replace(parse_format(value)?).is_some() {
                                return Err("格式重复".into());
                            }
                        }
                    }
                }
                value if value.starts_with("--format=") => {
                    if format.replace(parse_format(&value[9..])?).is_some() {
                        return Err("格式重复".into());
                    }
                }
                value if !value.starts_with('-') && file.is_none() => {
                    file = Some(PathBuf::from(value))
                }
                _ => return Err("lint shell参数无效".into()),
            }
            i += 1;
        }
        Ok(Self {
            file: file.ok_or("必须指定Shell文件")?,
            tool,
            config,
            dialect: dialect.ok_or("必须显式指定--dialect")?,
            json: format.unwrap_or(false),
            timeout,
        })
    }
}
fn parse_format(value: &str) -> Result<bool, String> {
    match value {
        "json" => Ok(true),
        "human" => Ok(false),
        _ => Err("格式仅支持human|json".into()),
    }
}
