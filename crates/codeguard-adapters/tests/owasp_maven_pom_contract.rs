use codeguard_adapters::{dependency_pom_direct_replay_eligible, owasp_maven_pom_plan};

#[test]
fn same_static_pom_can_enable_both_native_dependency_and_owasp_probes() {
    let pom = br#"<project><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>app</artifactId><version>1</version><build><plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-dependency-plugin</artifactId><version>3.8.1</version></plugin><plugin><groupId>org.owasp</groupId><artifactId>dependency-check-maven</artifactId><version>12.1.0</version></plugin></plugins></build><dependencies><dependency><groupId>org.demo</groupId><artifactId>lib</artifactId><version>2.0</version></dependency></dependencies></project>"#;
    assert!(dependency_pom_direct_replay_eligible(pom));
    assert_eq!(owasp_maven_pom_plan(pom).unwrap().plugin_version, "12.1.0");
    let duplicated = String::from_utf8(pom.to_vec()).unwrap().replace("</plugins>", "<plugin><groupId>org.owasp</groupId><artifactId>dependency-check-maven</artifactId><version>12.1.0</version></plugin></plugins>");
    assert!(!dependency_pom_direct_replay_eligible(
        duplicated.as_bytes()
    ));
    assert!(owasp_maven_pom_plan(duplicated.as_bytes()).is_none());
}

#[test]
fn direct_pinned_owasp_plugin_and_dependencies_are_replay_candidates() {
    let pom = br#"<project><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>app</artifactId><version>1.0</version><build><plugins><plugin><groupId>org.owasp</groupId><artifactId>dependency-check-maven</artifactId><version>12.1.0</version></plugin></plugins></build><dependencies><dependency><groupId>org.demo</groupId><artifactId>lib</artifactId><version>2.0</version></dependency></dependencies></project>"#;
    let plan = owasp_maven_pom_plan(pom).expect("simple pinned project");
    assert_eq!(plan.group_id, "demo");
    assert_eq!(plan.artifact_id, "app");
    assert_eq!(plan.version, "1.0");
    assert_eq!(plan.plugin_version, "12.1.0");
}

#[test]
fn inherited_dynamic_or_checker_configuration_is_not_silently_replayed() {
    let base = "<project><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>app</artifactId><version>1</version><build><plugins><plugin><groupId>org.owasp</groupId><artifactId>dependency-check-maven</artifactId><version>12.1.0</version></plugin></plugins></build></project>";
    for bad in [
        base.replace("<version>12.1.0</version>", ""),
        base.replace("12.1.0", "${odc.version}"),
        base.replace("<build>", "<parent><groupId>x</groupId></parent><build>"),
        base.replace("</plugin>", "<configuration><skip>true</skip></configuration></plugin>"),
        base.replace("</plugin>", "<configuration><excludes>x</excludes></configuration></plugin>"),
        base.replace("</plugins>", "<plugin><artifactId>other</artifactId></plugin></plugins>"),
        base.replace("</project>", "<profiles><profile/></profiles></project>"),
        base.replace("</project>", "<dependencies><dependency><groupId>x</groupId><artifactId>y</artifactId><version>[1,2)</version></dependency></dependencies></project>"),
        base.replace("</project>", "<repositories><repository/></repositories></project>"),
    ] {
        assert!(owasp_maven_pom_plan(bad.as_bytes()).is_none(), "{bad}");
    }
}

#[test]
fn malformed_or_duplicate_coordinates_do_not_create_a_plan() {
    for pom in [
        b"".as_slice(),
        b"<!DOCTYPE project><project/>".as_slice(),
        br#"<project><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><groupId>other</groupId><artifactId>app</artifactId><version>1</version><build><plugins><plugin><groupId>org.owasp</groupId><artifactId>dependency-check-maven</artifactId><version>12.1.0</version></plugin></plugins></build></project>"#.as_slice(),
    ] {
        assert!(owasp_maven_pom_plan(pom).is_none());
    }
}
