use codeguard_adapters::parse_maven_dependency_tree_json;

const TREE: &str = r#"{
  "groupId":"example", "artifactId":"app", "version":"1.0", "type":"jar",
  "scope":"", "classifier":"", "optional":"false",
  "children":[{
    "groupId":"org.demo", "artifactId":"client", "version":"2.1", "type":"jar",
    "scope":"compile", "classifier":"", "optional":"false",
    "children":[{
      "groupId":"org.demo", "artifactId":"core", "version":"3.0", "type":"jar",
      "scope":"compile", "classifier":"", "optional":"false"
    }]
  }]
}"#;

#[test]
fn parses_native_json_as_rooted_graph_without_claiming_vulnerability_status() {
    let tree = parse_maven_dependency_tree_json(TREE.as_bytes()).unwrap();
    assert_eq!(tree.nodes.len(), 3);
    assert_eq!(tree.edges, vec![(0, 1), (1, 2)]);
    assert_eq!(tree.nodes[2].group_id, "org.demo");
    assert_eq!(tree.nodes[2].version, "3.0");
    assert!(!tree.nodes[2].optional);
}

#[test]
fn empty_native_tree_is_a_graph_only_when_root_identity_is_complete() {
    let tree = parse_maven_dependency_tree_json(br#"{"groupId":"a","artifactId":"b","version":"1","type":"jar","scope":"","classifier":"","optional":"false"}"#).unwrap();
    assert_eq!(tree.nodes.len(), 1);
    assert!(tree.edges.is_empty());
    let bad = TREE.replace("\"version\":\"1.0\",", "");
    assert!(parse_maven_dependency_tree_json(bad.as_bytes()).is_err());
}

#[test]
fn rejects_dynamic_coordinates_and_non_native_shapes() {
    for broken in [
        TREE.replace("\"version\":\"3.0\"", "\"version\":\"${revision}\""),
        TREE.replace("\"optional\":\"false\"", "\"optional\":false"),
        TREE.replace("\"scope\":\"compile\"", "\"scope\":\"mystery\""),
        TREE.replace(
            "\"artifactId\":\"client\"",
            "\"artifactId\":\"client\\nother\"",
        ),
        TREE.replace("\"classifier\":\"\"", "\"classifier\":\"*\""),
        TREE.replace(
            "\"groupId\":\"example\"",
            "\"groupId\":\"example\",\"unexpected\":1",
        ),
    ] {
        assert!(
            parse_maven_dependency_tree_json(broken.as_bytes()).is_err(),
            "{broken}"
        );
    }
}

#[test]
fn rejects_oversized_and_excessively_deep_reports() {
    assert!(parse_maven_dependency_tree_json(&vec![b' '; 4 * 1024 * 1024 + 1]).is_err());
    let mut deep = String::new();
    for _ in 0..70 {
        deep.push_str("{\"groupId\":\"a\",\"artifactId\":\"b\",\"version\":\"1\",\"type\":\"jar\",\"scope\":\"compile\",\"classifier\":\"\",\"optional\":\"false\",\"children\":[");
    }
    deep.push_str(TREE);
    for _ in 0..70 {
        deep.push_str("]}");
    }
    assert!(parse_maven_dependency_tree_json(deep.as_bytes()).is_err());
}

#[test]
#[ignore = "requires explicit Maven 3.x and cached dependency plugin 3.8.1 plus JUnit artifacts"]
fn real_maven_dependency_tree_retains_transitive_edges() {
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    let maven = std::env::var("CODEGUARD_MAVEN_BIN").unwrap();
    let root = std::env::temp_dir().join(format!(
        "cg-native-dependency-tree-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("pom.xml"),
        r#"<project xmlns="http://maven.apache.org/POM/4.0.0"><modelVersion>4.0.0</modelVersion><groupId>cg.test</groupId><artifactId>tree-fixture</artifactId><version>1.0</version><build><plugins><plugin><artifactId>maven-dependency-plugin</artifactId><version>3.8.1</version></plugin></plugins></build><dependencies><dependency><groupId>junit</groupId><artifactId>junit</artifactId><version>4.13.2</version><scope>test</scope></dependency></dependencies></project>"#,
    )
    .unwrap();
    let result = Command::new(maven)
        .current_dir(&root)
        .args([
            "-B",
            "-ntp",
            "-o",
            "org.apache.maven.plugins:maven-dependency-plugin:3.8.1:tree",
            "-DoutputType=json",
            "-DoutputFile=tree.json",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    let tree =
        parse_maven_dependency_tree_json(&fs::read(root.join("tree.json")).unwrap()).unwrap();
    assert_eq!(tree.nodes.len(), 3);
    assert_eq!(tree.edges, vec![(0, 1), (1, 2)]);
    assert_eq!(tree.nodes[1].artifact_id, "junit");
    assert_eq!(tree.nodes[2].artifact_id, "hamcrest-core");
    fs::remove_dir_all(root).unwrap();
}
