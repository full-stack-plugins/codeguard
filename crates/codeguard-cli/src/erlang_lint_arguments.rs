use std::path::PathBuf;

/// Erlang 单文件语法观察参数；无效输入必须在启动原生进程前拒绝。
pub(crate) struct ErlangLintArguments {
    /// 普通源码文件。
    pub source: PathBuf,
    /// 显式 OTP 28 erl 工具，不从 PATH 猜测。
    pub erl_tool: Option<PathBuf>,
    /// 是否输出封闭 JSON 反馈。
    pub json: bool,
    /// 本轮共同超时预算及来源。
    pub timeout: (u64, &'static str),
}

impl ErlangLintArguments {
    /// 解析源码、工具、格式与预算；返回参数或不产生执行副作用的用法错误。
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut source = None;
        let mut erl_tool = None;
        let mut format = None;
        let mut timeout = None;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--erl-tool" => {
                    index += 1;
                    let value = args.get(index).ok_or("--erl-tool 缺少路径")?;
                    if erl_tool.replace(PathBuf::from(value)).is_some() {
                        return Err("--erl-tool 重复".into());
                    }
                }
                "--timeout" => {
                    index += 1;
                    let value = args.get(index).ok_or("--timeout 缺少预算")?;
                    if timeout
                        .replace(crate::check_budget::parse_check_timeout(value)?)
                        .is_some()
                    {
                        return Err("--timeout 重复".into());
                    }
                }
                "--format" => {
                    index += 1;
                    let value = args.get(index).ok_or("--format 缺少格式")?;
                    set_format(&mut format, value)?;
                }
                value if value.starts_with("--format=") => {
                    set_format(&mut format, &value[9..])?;
                }
                value if !value.starts_with('-') && source.is_none() => {
                    source = Some(PathBuf::from(value));
                }
                _ => return Err("lint erlang 参数无效".into()),
            }
            index += 1;
        }
        Ok(Self {
            source: source.ok_or("lint erlang 缺少源码文件")?,
            erl_tool,
            json: format.unwrap_or(false),
            timeout: crate::check_budget::select_check_timeout(timeout)?,
        })
    }
}

fn set_format(selected: &mut Option<bool>, value: &str) -> Result<(), String> {
    let json = match value {
        "json" => true,
        "human" => false,
        _ => return Err("--format 仅支持 human/json".into()),
    };
    if selected.replace(json).is_some() {
        return Err("--format 重复".into());
    }
    Ok(())
}
