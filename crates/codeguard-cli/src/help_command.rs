//! 静态帮助入口；项目、PATH、原生工具和策略都不是帮助查询的输入。
use serde_json::json;
use std::process::ExitCode;

/// 查询全部或精确前缀的当前命令目录。
/// 参数只允许命令词与唯一human/json格式；返回操作退出码，不执行目标命令。
pub fn run(args: &[String]) -> ExitCode {
    let mut words = Vec::new();
    let mut format = None;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        let value = if arg == "--format" {
            index += 1;
            match args.get(index) {
                Some(value) => Some(value.as_str()),
                None => return invalid(),
            }
        } else {
            arg.strip_prefix("--format=")
        };
        if let Some(value) = value {
            if format.is_some() || !matches!(value, "human" | "json") {
                return invalid();
            }
            format = Some(value);
        } else if arg.starts_with('-') && arg != "--version" {
            return invalid();
        } else {
            words.push(arg);
        }
        index += 1;
    }
    let selection = if words.is_empty() {
        None
    } else {
        Some(words.join(" "))
    };
    let commands = crate::command_catalogue::descriptors()
        .into_iter()
        .filter(|row| {
            selection
                .as_ref()
                .is_none_or(|command| row.command == command)
        })
        .collect::<Vec<_>>();
    if commands.is_empty() {
        return invalid();
    }
    if format == Some("json") {
        println!(
            "{}",
            json!({"schema_version":"0.2.0","report_type":"command_help",
            "operation":"help","cli_version":env!("CARGO_PKG_VERSION"),"command_status":"complete",
            "exit_code":0,"selection":selection,"native_execution":"not_run",
            "delivery_decision":"not_evaluated","commands":commands})
        );
    } else {
        println!(
            "codeguard {}\n用法: codeguard help COMMAND... [--format human|json]；省略COMMAND显示全部目录。",
            env!("CARGO_PKG_VERSION")
        );
        for row in commands {
            println!(
                "{} [{}] {}\n  {}",
                row.tracking_id.unwrap_or("扩展"),
                row.support,
                row.usage,
                row.scope
            );
            for example in row.examples {
                println!("  示例: {example}");
            }
        }
        println!(
            "帮助查询未执行目标命令；delivery_decision=not_evaluated。partial不等于完整验收；planned不可执行。语言专用工具参数见对应当前源码文档。"
        );
    }
    ExitCode::SUCCESS
}
fn invalid() -> ExitCode {
    eprintln!(
        "帮助参数无效：使用 help [COMMAND...] [--format human|json] 查询精确命令前缀，格式不可重复。"
    );
    ExitCode::from(2)
}

/// 识别精确命令前缀后的帮助参数；不吞掉路径、工具参数或其它输入。
/// 参数为完整argv；返回可交给静态帮助的前缀，复杂调用继续由原解析器拒绝/处理。
pub fn trailing_selection(args: &[String]) -> Option<Vec<String>> {
    if args.len() < 2 || !matches!(args.last()?.as_str(), "--help" | "-h") {
        return None;
    }
    let prefix = &args[..args.len() - 1];
    let selection = prefix.join(" ");
    crate::command_catalogue::descriptors()
        .iter()
        .any(|row| row.command == selection)
        .then(|| prefix.to_vec())
}
