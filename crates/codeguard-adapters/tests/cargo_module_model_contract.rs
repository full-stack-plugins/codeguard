use codeguard_adapters::CargoModuleModel;

#[test]
fn direct_members_and_renamed_paths_keep_dependency_scopes() {
    let model=CargoModuleModel::observe(b"[package]\nname='service'\n[workspace]\nmembers=['api']\n[dependencies]\nalias={package='real-api',path='../api'}\n[dev-dependencies]\nreal-api={path='../api'}\n[build-dependencies]\nreal-api={path='../api'}\n").unwrap();
    assert_eq!(model.package_name.as_deref(), Some("service"));
    assert_eq!(model.members, ["api"]);
    assert_eq!(model.path_dependencies.len(), 3);
    for scope in ["normal", "build", "dev"] {
        assert!(model.path_dependencies.contains(&(
            "real-api".into(),
            "../api".into(),
            scope.into()
        )));
    }
}

#[test]
fn workspace_conditions_and_external_sources_are_not_resolved_as_direct_paths() {
    let model=CargoModuleModel::observe(b"[workspace]\nmembers=['api','crates/*']\nexclude=['api']\n[dependencies]\none={workspace=true}\ntwo={path='api',optional=true}\nthree='1'\nfour={path='api',git='https://invalid.example'}\n[target.'cfg(unix)'.dependencies]\nfive={path='api'}\n[features]\ndefault=[]\n").unwrap();
    assert!(model.members.is_empty());
    assert!(model.path_dependencies.is_empty());
    for reason in [
        "workspace_member_unresolved",
        "workspace_exclusions_not_resolved",
        "dependency_workspace_not_resolved",
        "optional_dependency_not_resolved",
        "external_dependency_not_resolved",
        "dependency_source_not_resolved",
        "target_not_resolved",
        "features_not_resolved",
    ] {
        assert!(model.unresolved.contains(reason), "{reason}");
    }
}

#[test]
fn malformed_bounded_and_wrongly_typed_manifests_cannot_create_target_identity() {
    assert!(CargoModuleModel::observe(b"not TOML").is_err());
    assert!(CargoModuleModel::observe(b"name=1\nname=2").is_err());
    assert!(CargoModuleModel::observe(&vec![b' '; 256 * 1024 + 1]).is_err());
    assert!(CargoModuleModel::observe(&[255]).is_err());
    let model=CargoModuleModel::observe(b"[package]\nname={workspace=true}\n[dependencies]\na={path=1}\nb={path='api',package=1}\nc={path='api',optional='false'}\n").unwrap();
    assert!(model.package_name.is_none());
    assert!(model.path_dependencies.is_empty());
    for reason in [
        "package_name_unresolved",
        "dependency_path_not_resolved",
        "dependency_package_unresolved",
        "optional_dependency_not_resolved",
    ] {
        assert!(model.unresolved.contains(reason));
    }
}
