use codeguard_adapters::javadoc_pom_direct_replay_eligible;

const SIMPLE: &str = "<project><modelVersion>4.0.0</modelVersion><groupId>demo</groupId><artifactId>docs</artifactId><version>1</version><build><plugins><plugin><artifactId>maven-javadoc-plugin</artifactId><version>3.12.0</version><configuration><doclint>missing</doclint></configuration></plugin></plugins></build></project>";

#[test]
fn accepts_only_static_direct_javadoc_pom() {
    assert!(javadoc_pom_direct_replay_eligible(SIMPLE.as_bytes()));
    let namespaced = SIMPLE.replace(
        "<project>",
        "<project xmlns=\"http://maven.apache.org/POM/4.0.0\">",
    );
    assert!(javadoc_pom_direct_replay_eligible(namespaced.as_bytes()));
}

#[test]
fn inheritance_dynamic_configuration_and_extra_build_work_are_ineligible() {
    for changed in [
        SIMPLE.replace("<modelVersion>", "<parent/><modelVersion>"),
        SIMPLE.replace("<build>", "<dependencies/><build>"),
        SIMPLE.replace("<build>", "<profiles/><build>"),
        SIMPLE.replace("<build>", "<modules/><build>"),
        SIMPLE.replace("<plugins>", "<extensions/><plugins>"),
        SIMPLE.replace(
            "</plugin></plugins>",
            "</plugin><plugin><artifactId>maven-antrun-plugin</artifactId></plugin></plugins>",
        ),
        SIMPLE.replace("<configuration>", "<executions/><configuration>"),
        SIMPLE.replace(
            "<doclint>missing</doclint>",
            "<doclint>${codeguard.doclint}</doclint>",
        ),
        SIMPLE.replace("<doclint>missing</doclint>", "<doclint>none</doclint>"),
        SIMPLE.replace("<version>3.12.0</version>", "<version>3.11.0</version>"),
        SIMPLE.replace(
            "<doclint>missing</doclint>",
            "<doclint>missing</doclint><failOnError>false</failOnError>",
        ),
        SIMPLE.replace(
            "<doclint>missing</doclint>",
            "<doclint>missing</doclint><doclint>missing</doclint>",
        ),
    ] {
        assert!(
            !javadoc_pom_direct_replay_eligible(changed.as_bytes()),
            "{changed}"
        );
    }
}

#[test]
fn invalid_xml_attributes_and_foreign_namespace_are_ineligible() {
    for changed in [
        SIMPLE.replace("<project>", "<!DOCTYPE project><project>"),
        SIMPLE.replace("<project>", "<project data-custom=\"x\">"),
        SIMPLE.replace("<doclint>", "<doclint custom=\"x\">"),
        SIMPLE
            .replace("<doclint>", "<other:doclint xmlns:other=\"urn:other\">")
            .replace("</doclint>", "</other:doclint>"),
        SIMPLE.replace("</project>", ""),
    ] {
        assert!(
            !javadoc_pom_direct_replay_eligible(changed.as_bytes()),
            "{changed}"
        );
    }
}
