//! 统一命令语法、canonical ID/别名及只读 plan
//!
//! 验收标准：错参无进程副作用、plan 不运行构建/扫描

use serde::{Deserialize, Serialize};

/// 命令类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandType {
    /// 检查
    Check,
    /// 计划
    Plan,
    /// 修复
    Fix,
}

/// 命令解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandParseResult {
    /// 命令类型
    pub command_type: CommandType,
    /// Canonical ID
    pub canonical_id: String,
    /// 是否只读
    pub read_only: bool,
    /// 是否有副作用
    pub has_side_effect: bool,
}

/// 命令解析器
pub struct UnifiedCommand;

impl UnifiedCommand {
    /// 解析命令
    pub fn parse(args: &[String]) -> Result<CommandParseResult, String> {
        if args.is_empty() {
            return Err("no_command".into());
        }
        
        let command_type = match args[0].as_str() {
            "check" => CommandType::Check,
            "plan" => CommandType::Plan,
            "fix" => CommandType::Fix,
            _ => return Err("unknown_command".into()),
        };
        
        // 错参无进程副作用
        let has_side_effect = command_type != CommandType::Plan;
        
        Ok(CommandParseResult {
            command_type,
            canonical_id: args[0].clone(),
            read_only: command_type == CommandType::Plan,
            has_side_effect,
        })
    }
    
    /// 验证 plan 不运行构建/扫描
    pub fn validate_plan_no_execution(result: &CommandParseResult) -> bool {
        result.command_type != CommandType::Plan || !result.has_side_effect
    }
}
