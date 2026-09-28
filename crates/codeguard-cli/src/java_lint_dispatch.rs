//! `lint java` 的检查器选择；各适配器仍独立保留原生证据与未完成边界。

use std::process::ExitCode;

use crate::{java_checkstyle_command, java_javadoc_command, java_p3c_command};

/// 选择局部 Java 原生检查器；缺省仍为既有 P3C 单文件诊断。
pub fn run(args: &[String]) -> ExitCode {
    if args.is_empty() {
        return java_p3c_command::run(args);
    }
    let mut forwarded = Vec::with_capacity(args.len());
    forwarded.push(args[0].clone());
    let mut checker = None;
    let mut index = 1;
    while index < args.len() {
        let option = args[index].as_str();
        if option == "--checker" || option.starts_with("--checker=") {
            if checker.is_some() {
                eprintln!("lint java --checker 不得重复");
                return ExitCode::from(2);
            }
            let value = if let Some(value) = option.strip_prefix("--checker=") {
                value
            } else {
                index += 1;
                let Some(value) = args.get(index) else {
                    eprintln!("lint java --checker 缺少值");
                    return ExitCode::from(2);
                };
                value.as_str()
            };
            if !matches!(value, "p3c" | "javadoc" | "checkstyle") {
                eprintln!("lint java --checker 仅支持 p3c、javadoc 或 checkstyle");
                return ExitCode::from(2);
            }
            checker = Some(value.to_owned());
        } else {
            forwarded.push(args[index].clone());
        }
        index += 1;
    }
    match checker.as_deref() {
        Some("javadoc") => java_javadoc_command::run(&forwarded),
        Some("checkstyle") => java_checkstyle_command::run(&forwarded),
        _ => java_p3c_command::run(&forwarded),
    }
}
