//! 宿主事件到 Rust 核心路由的有界只读入口；当前只规划，不运行检查器。

use codeguard_core::{HookTriggerInput, plan_hook_trigger};
use serde_json::{Value, json};
use std::io::Read;
use std::process::ExitCode;

const MAX_REQUEST_BYTES: u64 = 64 * 1024;

/// 从 stdin 接收固定版本事件并输出候选检查档位；返回 3 表示检查尚未执行。
/// 参数仅支持 `--format=json`；无论规划结果如何都不签发质量或交付结论。
pub fn run(args: &[String]) -> ExitCode {
    if !args.is_empty() && !matches!(args, [format] if format == "--format=json") {
        eprintln!("hook plan 仅支持 --format=json；请求从 stdin 读取");
        return ExitCode::from(2);
    }
    let mut raw = Vec::new();
    match std::io::stdin()
        .lock()
        .take(MAX_REQUEST_BYTES + 1)
        .read_to_end(&mut raw)
    {
        Ok(_) if raw.is_empty() || raw.len() as u64 > MAX_REQUEST_BYTES => {
            eprintln!("hook 请求为空或超过 64 KiB");
            return ExitCode::from(2);
        }
        Ok(_) => {}
        Err(error) => {
            eprintln!("读取 hook 请求失败：{error}");
            return ExitCode::from(4);
        }
    }
    let input = match parse_request(&raw) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("无效 hook 请求：{error}");
            return ExitCode::from(2);
        }
    };
    let plan = match plan_hook_trigger(&input) {
        Ok(plan) => plan,
        Err(error) => {
            eprintln!("无效 hook 事件：{error}");
            return ExitCode::from(2);
        }
    };
    println!(
        "{}",
        json!({
            "schema_version": "1.1.0",
            "report_type": "hook_trigger_plan",
            "planning_status": "candidate",
            "execution": "not_run",
            "delivery_decision": "not_evaluated",
            "plan": plan,
        })
    );
    ExitCode::from(3)
}

/// 解析原始 `raw` 宿主请求；返回版本化事件或严格协议错误。
pub(crate) fn parse_request(raw: &[u8]) -> Result<HookTriggerInput, String> {
    let value = codeguard_adapters::parse_unique_json(raw).map_err(str::to_owned)?;
    let object = value.as_object().ok_or("hook_request_not_object")?;
    if object.len() != 3
        || object.get("schema_version") != Some(&Value::String("1.0.0".into()))
        || object.get("report_type") != Some(&Value::String("hook_trigger_request".into()))
    {
        return Err("hook_request_version_or_fields_invalid".into());
    }
    let input = object.get("input").ok_or("hook_input_missing")?;
    serde_json::from_value(input.clone()).map_err(|error| error.to_string())
}
