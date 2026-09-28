use codeguard_adapters::parse_npm_audit_json;
use serde_json::{Value, json};
fn report() -> Value {
    json!({"auditReportVersion":2,"vulnerabilities":{"fixture-pkg":{"name":"fixture-pkg","severity":"high","isDirect":true,"via":[{"source":123,"name":"fixture-pkg","dependency":"fixture-pkg","title":"fixture advisory","url":"https://example.invalid/advisory/123","severity":"high","range":"<2.0.0"}],"effects":[],"range":"<2.0.0","nodes":["node_modules/fixture-pkg"],"fixAvailable":false}},"metadata":{"vulnerabilities":{"info":0,"low":0,"moderate":0,"high":1,"critical":0,"total":1},"dependencies":{"prod":2,"dev":0,"optional":0,"peer":0,"peerOptional":0,"total":1}}})
}
#[test]
fn native_advisory_is_not_confused_with_exit_failure_or_a_resolved_version() {
    let native = report();
    let observed =
        parse_npm_audit_json(native.to_string().as_bytes(), "11.16.0", "11.16.0", Some(1)).unwrap();
    assert_eq!(observed.advisory_coverage, "not_evaluated");
    assert_eq!(observed.components.len(), 1);
    assert_eq!(observed.components[0].native_name, "fixture-pkg");
    assert_eq!(observed.components[0].advisory_sources, vec![123]);
    assert_eq!(observed.components[0].affected_range, "<2.0.0");
    assert!(
        parse_npm_audit_json(native.to_string().as_bytes(), "11.16.0", "11.16.0", Some(0)).is_err()
    );
}
#[test]
fn native_errors_bad_counts_versions_and_paths_never_become_clean_audits() {
    let mut cases = Vec::new();
    let mut r = report();
    r["metadata"]["vulnerabilities"]["total"] = json!(0);
    cases.push(r);
    let mut r = report();
    r["vulnerabilities"]["fixture-pkg"]["nodes"] = json!(["../outside"]);
    cases.push(r);
    let mut r = report();
    r["vulnerabilities"]["fixture-pkg"]["name"] = json!("other");
    cases.push(r);
    let mut r = report();
    r["vulnerabilities"]["fixture-pkg"]["via"] = json!(["missing-component"]);
    cases.push(r);
    let mut r = report();
    r["vulnerabilities"]["fixture-pkg"]["severity"] = json!("low");
    r["metadata"]["vulnerabilities"]["high"] = json!(0);
    r["metadata"]["vulnerabilities"]["low"] = json!(1);
    cases.push(r);
    cases.push(json!({"error":{"code":"ENOLOCK","summary":"need lock"}}));
    for r in cases {
        assert!(
            parse_npm_audit_json(r.to_string().as_bytes(), "11.16.0", "11.16.0", Some(1)).is_err(),
            "{r}"
        );
    }
    assert!(
        parse_npm_audit_json(
            report().to_string().as_bytes(),
            "11.16.0",
            "11.15.0",
            Some(1)
        )
        .is_err()
    );
    assert!(parse_npm_audit_json(br#"{"auditReportVersion":2,"auditReportVersion":2,"vulnerabilities":{},"metadata":{}}"#,"11.16.0","11.16.0",Some(0)).is_err());
}

#[test]
fn indirect_native_relationships_do_not_forge_advisory_ids() {
    let mut r = report();
    let mut wrapper = r["vulnerabilities"]["fixture-pkg"].clone();
    wrapper["name"] = json!("wrapper");
    wrapper["via"] = json!(["fixture-pkg"]);
    wrapper["nodes"] = json!(["node_modules/wrapper"]);
    r["vulnerabilities"]["wrapper"] = wrapper;
    r["metadata"]["vulnerabilities"]["high"] = json!(2);
    r["metadata"]["vulnerabilities"]["total"] = json!(2);
    let observed =
        parse_npm_audit_json(r.to_string().as_bytes(), "11.16.0", "11.16.0", Some(1)).unwrap();
    let wrapper = observed
        .components
        .iter()
        .find(|c| c.native_name == "wrapper")
        .unwrap();
    assert!(wrapper.advisory_sources.is_empty());
    assert_eq!(wrapper.via_components, vec!["fixture-pkg"]);
}

#[test]
fn audit_plan_retains_literal_paths_without_fix_install_or_network() {
    let mut c = codeguard_adapters::NpmAuditCommand {
        node: "/tmp/node".into(),
        entry: "/tmp/npm cli.js".into(),
        cache: "/tmp/cache;$(touch ignored)".into(),
        user_config: "/tmp/user.npmrc".into(),
        global_config: "/tmp/global.npmrc".into(),
    };
    let args = c.args().unwrap();
    assert_eq!(args[1], "/tmp/npm cli.js");
    assert!(args.iter().any(|a| a == "--offline"));
    assert!(!args.iter().any(|a| a == "fix" || a == "install"));
    c.cache = c.entry.clone();
    assert!(c.args().is_err());
}

#[test]
fn audit_configuration_requires_an_explicit_declaration_without_executing_scripts() {
    for (raw, state, reason) in [
        (
            r#"{"scripts":{"security":"npm audit --json"}}"#,
            "configured",
            "npm_audit_runtime_context_unverified",
        ),
        (
            r#"{"scripts":{"audit":"node audit.js"}}"#,
            "missing",
            "npm_audit_script_not_observed",
        ),
        (
            r#"{"scripts":{"audit":"npm audit && echo done"}}"#,
            "unknown",
            "npm_audit_custom_invocation_requires_review",
        ),
        (
            r#"{"scripts":{"audit":"npm audit"},"scripts":{}}"#,
            "unknown",
            "npm_manifest_invalid",
        ),
        (r#"{"scripts":true}"#, "invalid", "npm_scripts_invalid"),
    ] {
        let observed = codeguard_adapters::inspect_npm_audit_config(
            raw.as_bytes(),
            "packages/app",
            "packages/app/package.json",
        );
        assert_eq!(observed.configuration, state);
        assert_eq!(observed.reason, reason);
        assert_eq!(observed.category, "cve");
    }
}

#[test]
fn online_audit_requires_an_explicit_registry_without_embedded_credentials() {
    let c = codeguard_adapters::NpmAuditCommand {
        node: "/tmp/node".into(),
        entry: "/tmp/npm.js".into(),
        cache: "/tmp/cache".into(),
        user_config: "/tmp/user.npmrc".into(),
        global_config: "/tmp/global.npmrc".into(),
    };
    let args = c.args_for_registry("https://registry.npmjs.org/").unwrap();
    assert!(!args.iter().any(|a| a == "--offline"));
    assert!(
        args.iter()
            .any(|a| a == "--registry=https://registry.npmjs.org/")
    );
    assert!(
        args.iter()
            .any(|a| a == "--audit-registry=https://registry.npmjs.org/")
    );
    assert!(c.args_for_registry("http://127.0.0.1:12345/").is_ok());
    for bad in [
        "",
        "http://example.com/",
        "https://user:token@example.com/",
        "https://example.com/?token=a",
        "https://example.com/#x",
        "https://example.com/a",
        "https://example.com/\n--force",
        "https://example.com:0/",
        "https://example.com:65536/",
    ] {
        assert!(c.args_for_registry(bad).is_err(), "{bad:?}");
    }
}
