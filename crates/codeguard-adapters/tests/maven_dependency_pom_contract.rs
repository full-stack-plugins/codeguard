use codeguard_adapters::dependency_pom_direct_replay_eligible;

const SAFE: &str = r#"<project xmlns="http://maven.apache.org/POM/4.0.0"><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>app</artifactId><version>1</version><build><plugins><plugin><artifactId>maven-dependency-plugin</artifactId><version>3.8.1</version></plugin></plugins></build><dependencies><dependency><groupId>junit</groupId><artifactId>junit</artifactId><version>4.13.2</version><scope>test</scope></dependency></dependencies></project>"#;

#[test]
fn direct_static_dependency_project_is_replayable() {
    assert!(dependency_pom_direct_replay_eligible(SAFE.as_bytes()));
}

#[test]
fn inherited_dynamic_or_active_pom_is_not_replayable() {
    for changed in [
        SAFE.replace("<dependencies>", "<parent><groupId>x</groupId><artifactId>p</artifactId><version>1</version></parent><dependencies>"),
        SAFE.replace("<dependencies>", "<repositories><repository><id>x</id><url>https://example.com</url></repository></repositories><dependencies>"),
        SAFE.replace("<dependencies>", "<profiles><profile><id>x</id></profile></profiles><dependencies>"),
        SAFE.replace("<dependencies>", "<modules><module>other</module></modules><dependencies>"),
        SAFE.replace("<dependencies>", "<build><extensions><extension/></extensions></build><dependencies>"),
        SAFE.replace("4.13.2", "${junit.version}"),
        SAFE.replace("<scope>test</scope>", "<scope>system</scope><systemPath>/tmp/local.jar</systemPath>"),
        SAFE.replace("<version>3.8.1</version>", "<version>3.6.0</version>"),
        SAFE.replace("<artifactId>maven-dependency-plugin</artifactId>", "<artifactId>maven-antrun-plugin</artifactId>"),
        SAFE.replace("<plugin>", "<other>").replace("</plugin>", "</other>"),
    ] {
        assert!(!dependency_pom_direct_replay_eligible(changed.as_bytes()), "{changed}");
    }
}
