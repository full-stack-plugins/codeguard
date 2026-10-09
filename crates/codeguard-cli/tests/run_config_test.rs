//! 运行配置测试

use codeguard_cli::run_config::*;

fn create_policy() -> ApprovedPolicy {
    ApprovedPolicy {
        required: vec!["lint".into(), "comments".into()],
        min_threshold: 0.98,
        exclude: vec!["vendor/".into()],
        version: "1.0.0".into(),
    }
}

fn create_run_config() -> RunConfig {
    RunConfig {
        version: "1.0.0".into(),
        timeout_ms: 5000,
        jobs: 4,
    }
}

#[test]
fn resolve_merges_run_config() {
    let policy = create_policy();
    let cli = create_run_config();
    let env = RunConfig { version: "1.0.0".into(), timeout_ms: 10000, jobs: 8 };
    
    let merged = ConfigResolver::resolve(&policy, &cli, &env);
    
    // CLI 优先
    assert_eq!(merged.run.timeout_ms, 5000);
    assert_eq!(merged.run.jobs, 4);
}

#[test]
fn resolve_cannot_weaken_policy() {
    let policy = create_policy();
    let cli = create_run_config();
    let env = create_run_config();
    
    let merged = ConfigResolver::resolve(&policy, &cli, &env);
    
    // 策略不被弱化
    assert_eq!(merged.policy.required, vec!["lint".to_string(), "comments".to_string()]);
    assert_eq!(merged.policy.min_threshold, 0.98);
    assert_eq!(merged.policy.exclude, vec!["vendor/".to_string()]);
}

#[test]
fn validate_rejects_empty_required() {
    let policy = ApprovedPolicy {
        required: vec![],
        min_threshold: 0.98,
        exclude: vec![],
        version: "1.0.0".into(),
    };
    let run = create_run_config();
    let merged = ConfigResolver::resolve(&policy, &run, &run);
    
    assert!(ConfigResolver::validate(&merged).is_err());
}

#[test]
fn validate_rejects_bad_threshold() {
    let policy = ApprovedPolicy {
        required: vec!["lint".into()],
        min_threshold: 1.5,
        exclude: vec![],
        version: "1.0.0".into(),
    };
    let run = create_run_config();
    let merged = ConfigResolver::resolve(&policy, &run, &run);
    
    assert!(ConfigResolver::validate(&merged).is_err());
}

#[test]
fn validate_accepts_good_config() {
    let policy = create_policy();
    let run = create_run_config();
    let merged = ConfigResolver::resolve(&policy, &run, &run);
    
    assert!(ConfigResolver::validate(&merged).is_ok());
}

#[test]
fn explain_shows_config() {
    let policy = create_policy();
    let run = create_run_config();
    let merged = ConfigResolver::resolve(&policy, &run, &run);
    
    let explanation = ConfigResolver::explain(&merged);
    assert_eq!(explanation.required_count, 2);
    assert_eq!(explanation.min_threshold, 0.98);
    assert!(!explanation.weakening_attempted);
}

#[test]
fn cli_env_cannot_weaken_threshold() {
    let policy = create_policy();
    // CLI 尝试降低阈值（但这是 RunConfig，不包含 threshold）
    let cli = RunConfig { version: "1.0.0".into(), timeout_ms: 5000, jobs: 4 };
    let env = RunConfig { version: "1.0.0".into(), timeout_ms: 5000, jobs: 4 };
    
    let merged = ConfigResolver::resolve(&policy, &cli, &env);
    
    // 阈值仍为批准值
    assert_eq!(merged.policy.min_threshold, 0.98);
}

#[test]
fn cli_env_cannot_weaken_required() {
    let policy = create_policy();
    let cli = create_run_config();
    let env = create_run_config();
    
    let merged = ConfigResolver::resolve(&policy, &cli, &env);
    
    // required 仍为批准值
    assert_eq!(merged.policy.required.len(), 2);
}

#[test]
fn cli_env_cannot_weaken_exclude() {
    let policy = create_policy();
    let cli = create_run_config();
    let env = create_run_config();
    
    let merged = ConfigResolver::resolve(&policy, &cli, &env);
    
    // exclude 仍为批准值
    assert_eq!(merged.policy.exclude.len(), 1);
}
