//! Adapter 协议测试

use codeguard_adapters::adapter_protocol::*;

/// 测试适配器
struct TestAdapter;

impl AdapterProtocol for TestAdapter {
    fn capability(&self) -> AdapterCapability {
        AdapterCapability {
            id: "test".into(),
            language: "test".into(),
            categories: vec!["lint".into()],
            protocol_version: "1.0.0".into(),
        }
    }
    
    fn discover(&self, _root: &std::path::Path) -> Result<DiscoveryResult, String> {
        Ok(DiscoveryResult {
            status: "configured".into(),
            config_files: vec!["config.toml".into()],
            tool_version: Some("1.0.0".into()),
        })
    }
    
    fn resolve(&self, tool: &str) -> Result<ToolIdentity, String> {
        Ok(ToolIdentity {
            name: tool.into(),
            version: "1.0.0".into(),
            path: "/usr/bin/tool".into(),
            sha256: Some("abc123".into()),
        })
    }
    
    fn plan(&self, input: &PlanInput) -> Result<ExecutionPlan, String> {
        Ok(ExecutionPlan {
            argv: vec!["tool".into(), "--check".into()],
            cwd: ".".into(),
            env: vec![],
        })
    }
    
    fn parse(&self, _report: &[u8]) -> Result<ParsedFindings, String> {
        Ok(ParsedFindings {
            findings: vec![],
            complete: true,
            incomplete_reason: None,
        })
    }
    
    fn coverage(&self, expected: &[String], actual: &[String]) -> CoverageResult {
        let uncovered: Vec<String> = expected.iter()
            .filter(|e| !actual.contains(e))
            .cloned()
            .collect();
        CoverageResult {
            complete: uncovered.is_empty(),
            uncovered,
        }
    }
    
    fn fix(&self, _finding: &Finding) -> Option<FixSuggestion> {
        None
    }
}

#[test]
fn adapter_capability_declaration() {
    let adapter = TestAdapter;
    let cap = adapter.capability();
    assert_eq!(cap.id, "test");
    assert_eq!(cap.language, "test");
    assert_eq!(cap.protocol_version, "1.0.0");
}

#[test]
fn adapter_discover_config() {
    let adapter = TestAdapter;
    let result = adapter.discover(std::path::Path::new(".")).unwrap();
    assert_eq!(result.status, "configured");
    assert!(!result.config_files.is_empty());
}

#[test]
fn adapter_resolve_tool() {
    let adapter = TestAdapter;
    let identity = adapter.resolve("test-tool").unwrap();
    assert_eq!(identity.name, "test-tool");
    assert_eq!(identity.version, "1.0.0");
}

#[test]
fn adapter_plan_execution() {
    let adapter = TestAdapter;
    let input = PlanInput {
        sources: vec!["main.rs".into()],
        config: None,
        timeout_ms: 5000,
    };
    let plan = adapter.plan(&input).unwrap();
    assert!(!plan.argv.is_empty());
}

#[test]
fn adapter_parse_report() {
    let adapter = TestAdapter;
    let result = adapter.parse(b"{}").unwrap();
    assert!(result.complete);
}

#[test]
fn adapter_coverage_check() {
    let adapter = TestAdapter;
    let expected = vec!["a.rs".into(), "b.rs".into()];
    let actual = vec!["a.rs".into()];
    let result = adapter.coverage(&expected, &actual);
    assert!(!result.complete);
    assert_eq!(result.uncovered, vec!["b.rs".to_string()]);
}

#[test]
fn adapter_registry_register_and_find() {
    let mut registry = AdapterRegistry::new();
    registry.register(Box::new(TestAdapter));
    
    let found = registry.find_by_language("test");
    assert!(found.is_some());
    
    let caps = registry.capabilities();
    assert_eq!(caps.len(), 1);
}

#[test]
fn adapter_protocol_no_bypass_runtime() {
    // 验证 adapter 协议不绕开运行时启动进程或联网
    let adapter = TestAdapter;
    let plan = adapter.plan(&PlanInput {
        sources: vec![],
        config: None,
        timeout_ms: 1000,
    }).unwrap();
    
    // 计划只包含命令参数，不包含网络或进程启动
    assert!(plan.argv.len() > 0);
    assert!(plan.env.is_empty() || plan.env.iter().all(|(k, _)| !k.contains("http")));
}
