//! 统一命令测试

use codeguard_cli::unified_command::*;

#[test]
fn parse_check_command() {
    let args = vec!["check".to_string()];
    let result = UnifiedCommand::parse(&args).unwrap();
    assert_eq!(result.command_type, CommandType::Check);
    assert!(result.has_side_effect);
}

#[test]
fn parse_plan_command() {
    let args = vec!["plan".to_string()];
    let result = UnifiedCommand::parse(&args).unwrap();
    assert_eq!(result.command_type, CommandType::Plan);
    assert!(result.read_only);
    assert!(!result.has_side_effect);
}

#[test]
fn parse_unknown_command() {
    let args = vec!["unknown".to_string()];
    assert!(UnifiedCommand::parse(&args).is_err());
}

#[test]
fn validate_plan_no_execution() {
    let args = vec!["plan".to_string()];
    let result = UnifiedCommand::parse(&args).unwrap();
    assert!(UnifiedCommand::validate_plan_no_execution(&result));
}

#[test]
fn parse_empty_args() {
    let args = vec![];
    assert!(UnifiedCommand::parse(&args).is_err());
}
