use std::path::PathBuf;

/// Python 项目注释入口参数；不隐式选择规则或改写项目配置。
pub(crate) struct PythonCommentsArguments {
    pub root: PathBuf,
    pub tool: Option<PathBuf>,
    pub json: bool,
    pub timeout: Option<u64>,
}
impl PythonCommentsArguments {
    /// 解析语种后 argv，返回根目录、工具与预算；非法参数在执行工具前拒绝。
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let (mut root, mut tool, mut format, mut timeout) = (None, None, None, None);
        let mut index = 0;
        while index < args.len() {
            let word = args[index].as_str();
            let (key, value) = if word.starts_with("--") && word.contains('=') {
                word.split_once('=').expect("已经确认赋值分隔符")
            } else if matches!(word, "--ruff-tool" | "--format" | "--timeout") {
                index += 1;
                (word, args.get(index).ok_or("参数缺少值")?.as_str())
            } else {
                (word, "")
            };
            match key {
                "--ruff-tool" if tool.is_none() => {
                    let path = PathBuf::from(value);
                    if !path.is_absolute() {
                        return Err("--ruff-tool 必须为绝对路径".into());
                    }
                    tool = Some(path);
                }
                "--format" if format.is_none() => {
                    format = Some(match value {
                        "json" => true,
                        "human" => false,
                        _ => return Err("--format 仅支持 human|json".into()),
                    });
                }
                "--timeout" if timeout.is_none() => {
                    timeout = Some(crate::check_budget::parse_check_timeout(value)?)
                }
                value if !value.is_empty() && !value.starts_with('-') && root.is_none() => {
                    root = Some(PathBuf::from(word))
                }
                _ => return Err("comments python 参数非法或重复".into()),
            }
            index += 1;
        }
        Ok(Self {
            root: root.unwrap_or_else(|| PathBuf::from(".")),
            tool,
            json: format.unwrap_or(false),
            timeout,
        })
    }
}
