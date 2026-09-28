use codeguard_adapters::{CargoModuleModel, MavenModuleModel};

#[test]
fn java_direct_targets_keep_distinct_property_names_without_guessing_profiles() {
    let model=MavenModuleModel::observe(b"<project><properties><maven.compiler.release>17</maven.compiler.release><maven.compiler.source>1.8</maven.compiler.source><maven.compiler.target>11</maven.compiler.target></properties><profiles><profile><properties><maven.compiler.release>21</maven.compiler.release></properties></profile></profiles></project>").unwrap();
    assert_eq!(model.language_targets.len(), 3);
    assert_eq!(model.language_targets["maven.compiler.release"], "17");
    assert_eq!(model.language_targets["maven.compiler.source"], "1.8");
    assert_eq!(model.language_targets["maven.compiler.target"], "11");
    assert!(model.unresolved.contains("profiles_not_evaluated"));
}

#[test]
fn java_variables_duplicates_nested_and_segmented_values_cannot_become_targets() {
    for property in [
        "${java.version}",
        "0",
        "abc",
        "17<!--split-->extra",
        "<nested>17</nested>",
        "1.8",
    ] {
        let xml = format!(
            "<project><properties><maven.compiler.release>{property}</maven.compiler.release></properties></project>"
        );
        let model = MavenModuleModel::observe(xml.as_bytes()).unwrap();
        assert!(model.language_targets.is_empty(), "{property}");
        assert!(
            model
                .unresolved
                .contains("language_target_unresolved:maven.compiler.release")
        );
    }
    for xml in [
        "<project><properties><maven.compiler.release>17</maven.compiler.release><maven.compiler.release>21</maven.compiler.release></properties></project>",
        "<project><properties><maven.compiler.release>17</maven.compiler.release></properties><properties><maven.compiler.release>21</maven.compiler.release></properties></project>",
    ] {
        assert!(
            MavenModuleModel::observe(xml.as_bytes())
                .unwrap()
                .language_targets
                .is_empty()
        );
    }
}

#[test]
fn rust_direct_targets_are_not_package_versions_or_workspace_defaults() {
    let model=CargoModuleModel::observe(b"[package]\nname='app'\nversion='9.9.9'\nrust-version='1.85'\nedition='2024'\n[workspace.package]\nrust-version='1.99'\nedition='2021'\n").unwrap();
    assert_eq!(model.language_targets["rust-version"], "1.85");
    assert_eq!(model.language_targets["edition"], "2024");
    assert_eq!(model.language_targets.len(), 2);
    let missing = CargoModuleModel::observe(b"[package]\nname='app'\nversion='1.0.0'\n").unwrap();
    assert!(missing.language_targets.is_empty());
}

#[test]
fn rust_inherited_wrong_types_or_unknown_edition_require_resolution() {
    for fields in [
        "rust-version={workspace=true}\nedition={workspace=true}",
        "rust-version=1.85\nedition=2024",
        "rust-version='latest'\nedition='future'",
        "rust-version='1.'\nedition='2025'",
    ] {
        let text = format!("[package]\nname='app'\n{fields}\n");
        let model = CargoModuleModel::observe(text.as_bytes()).unwrap();
        assert!(model.language_targets.is_empty());
        for reason in [
            "language_target_unresolved:rust-version",
            "language_target_unresolved:edition",
        ] {
            assert!(model.unresolved.contains(reason));
        }
    }
}
