use codeguard_adapters::{NpmLockedNode, parse_npm_audit_json};
use serde_json::json;

#[test]
fn audit_nodes_bind_to_exact_locked_versions_without_guessing() {
    let lock = json!({"lockfileVersion":3,"packages":{"":{"name":"app","version":"1.0.0"},"node_modules/pkg":{"version":"1.2.3"},"node_modules/parent/node_modules/pkg":{"version":"1.1.0"}}});
    let nodes = NpmLockedNode::parse(lock.to_string().as_bytes()).unwrap();
    assert_eq!(nodes.len(), 2);
    let audit = json!({"auditReportVersion":2,"vulnerabilities":{"pkg":{"name":"pkg","severity":"high","isDirect":true,"via":[{"source":1,"name":"pkg","dependency":"pkg","title":"fixture","url":"https://example.invalid","severity":"high","range":"<2"}],"effects":[],"range":"<2","nodes":["node_modules/pkg","node_modules/parent/node_modules/pkg"],"fixAvailable":false}},"metadata":{"vulnerabilities":{"info":0,"low":0,"moderate":0,"high":1,"critical":0,"total":1},"dependencies":{"prod":3,"dev":0,"optional":0,"peer":0,"peerOptional":0,"total":2}}});
    let audit =
        parse_npm_audit_json(audit.to_string().as_bytes(), "11.16.0", "11.16.0", Some(1)).unwrap();
    let bound = NpmLockedNode::bind(&nodes, &audit).unwrap();
    assert_eq!(
        bound
            .iter()
            .map(|n| n.resolved_version.as_str())
            .collect::<Vec<_>>(),
        vec!["1.2.3", "1.1.0"]
    );
    assert!(NpmLockedNode::bind(&nodes[..1], &audit).is_err());
    let mut absent = nodes.clone();
    absent[0].location = "node_modules/missing".into();
    absent[0].native_name = "missing".into();
    assert!(NpmLockedNode::bind(&absent, &audit).is_err());
    let mut wrong = nodes;
    wrong[0].native_name = "other".into();
    assert!(NpmLockedNode::bind(&wrong, &audit).is_err());
}

#[test]
fn ambiguous_links_aliases_missing_versions_and_duplicate_keys_are_incomplete() {
    for raw in [
        r#"{"lockfileVersion":3,"packages":{"":{},"node_modules/p":{"link":true,"resolved":"packages/p"}}}"#,
        r#"{"lockfileVersion":3,"packages":{"":{},"node_modules/p":{"name":"other","version":"1.0.0"}}}"#,
        r#"{"lockfileVersion":3,"packages":{"":{},"node_modules/p":{}}}"#,
        r#"{"lockfileVersion":3,"packages":{"":{},"../p":{"version":"1.0.0"}}}"#,
        r#"{"lockfileVersion":3,"packages":{"":{},"node_modules/p":{"version":"1.0.0","version":"2.0.0"}}}"#,
        r#"{"lockfileVersion":2,"packages":{"":{}}}"#,
    ] {
        assert!(NpmLockedNode::parse(raw.as_bytes()).is_err(), "{raw}");
    }
    let scoped = br#"{"lockfileVersion":3,"packages":{"":{},"node_modules/@scope/pkg":{"version":"1.2.3"}}}"#;
    assert_eq!(
        NpmLockedNode::parse(scoped).unwrap()[0].native_name,
        "@scope/pkg"
    );
}
