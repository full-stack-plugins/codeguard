use codeguard_adapters::{CargoModuleModel, MavenModuleModel};

#[test]
fn cargo_versions_require_direct_semver_and_preserve_original_value() {
    for version in ["1.2.3", "1.2.3-rc.1+build.2"] {
        let input = format!("[package]\nname='app'\nversion='{version}'\n");
        let model = CargoModuleModel::observe(input.as_bytes()).unwrap();
        assert_eq!(model.package_version.as_deref(), Some(version));
    }
    for declaration in [
        "version={workspace=true}",
        "version=12",
        "version='latest'",
        "version='1.2'",
        "version='01.2.3'",
        "version='1.2.3-01'",
    ] {
        let input =
            format!("[package]\nname='app'\n{declaration}\n[workspace.package]\nversion='8.9.0'\n");
        let model = CargoModuleModel::observe(input.as_bytes()).unwrap();
        assert!(model.package_version.is_none(), "{declaration}");
        assert!(model.unresolved.contains("package_version_unresolved"));
    }
    let model =
        CargoModuleModel::observe(b"[package]\nname='app'\n[workspace.package]\nversion='8.9.0'\n")
            .unwrap();
    assert!(model.package_version.is_none());
    assert!(model.unresolved.contains("package_version_not_declared"));
}

#[test]
fn maven_version_is_independent_of_complete_coordinates_and_never_inherited() {
    let model =
        MavenModuleModel::observe(b"<project><version>2.3.4-SNAPSHOT</version></project>").unwrap();
    assert_eq!(model.package_version.as_deref(), Some("2.3.4-SNAPSHOT"));
    assert!(model.coordinates.is_none());
    for declaration in [
        "<version>${revision}</version>",
        "<version>1</version><version>2</version>",
        "<version><nested>1</nested></version>",
        "<version>1<!-- split -->.2</version>",
        "<version>1 2</version>",
        "<version>1/2</version>",
        "<version></version>",
    ] {
        let input =
            format!("<project>{declaration}<parent><version>8.9.0</version></parent></project>");
        let model = MavenModuleModel::observe(input.as_bytes()).unwrap();
        assert!(model.package_version.is_none(), "{declaration}");
        assert!(model.unresolved.contains("package_version_unresolved"));
    }
    for declaration in [
        "<parent><version>8.9.0</version></parent>",
        "<profiles><profile><version>8.9.0</version></profile></profiles>",
    ] {
        let input = format!("<project>{declaration}</project>");
        let model = MavenModuleModel::observe(input.as_bytes()).unwrap();
        assert!(model.package_version.is_none());
        assert!(model.unresolved.contains("package_version_not_declared"));
    }
}
