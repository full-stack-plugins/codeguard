//! 运行配置策略测试

use codeguard_cli::run_config_policy::*;

fn create_policy() -> ApprovedPolicy {
    ApprovedPolicy {
        required: vec!["lint".into()],
        min_threshold: 0.98,
        exclude: vec!["vendor/".into()],
    }
}

#[test]
fn resolve_config() {
    let policy = create_policy();
    let cli = RunConfig { timeout_ms: 5000, jobs: 4 };
    let env = RunConfig { timeout_ms: 10000, jobs: 8 };
    
    let merged = RunConfigPolicyResolver::resolve(&policy, &cli, &env);
    assert_eq!(merged.run.timeout_ms, 5000);
    assert_eq!(merged.run.jobs, 4);
}

#[test]
fn validate_no_weakening() {
    let policy = create_policy();
    let cli = RunConfig { timeout_ms: 5000, jobs: 4 };
    let env = RunConfig { timeout_ms: 5000, jobs: 4 };
    
    let merged = RunConfigPolicyResolver::resolve(&policy, &cli, &env);
    assert!(RunConfigPolicyResolver::validate_no_weakening(&merged, &policy));
}
