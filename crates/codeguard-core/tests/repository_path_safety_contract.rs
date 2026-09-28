use codeguard_core::check_repository_paths;

#[test]
fn hidden_secret_file_is_rejected_but_first_party_dot_directory_is_allowed() {
    let actual = check_repository_paths(&[
        ".env".into(),
        ".github/workflows/ci.yml".into(),
        ".github/keys/secret.pem".into(),
        "codeguard/tasks/CG-1.json".into(),
    ])
    .unwrap();
    assert_eq!(actual.len(), 2);
    assert_eq!(actual[0].path, ".env");
    assert_eq!(actual[1].path, ".github/keys/secret.pem");
}

#[test]
fn build_artifacts_and_root_dumps_are_rejected_without_nested_fixture_false_positives() {
    let actual = check_repository_paths(&[
        "target/site/index.html".into(),
        "vendor/lib/x.js".into(),
        "scripts/vendor/skill_vendor.py".into(),
        "tests/fixtures/sample.db".into(),
        "debug.log".into(),
        "codeguard/src/app.py".into(),
    ])
    .unwrap();
    assert_eq!(
        actual
            .iter()
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>(),
        ["target/site/index.html", "vendor/lib/x.js", "debug.log"]
    );
}

#[test]
fn ambiguous_or_noncanonical_git_paths_are_not_silently_accepted() {
    assert!(check_repository_paths(&["../outside/.env".into()]).is_err());
    assert!(check_repository_paths(&["a//b".into()]).is_err());
    assert!(check_repository_paths(&["a\\.env".into()]).is_err());
    assert!(check_repository_paths(&["\n".into()]).is_err());
}
