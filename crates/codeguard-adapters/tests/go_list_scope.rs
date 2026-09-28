use codeguard_adapters::parse_go_list_scope;
use std::path::Path;
#[test]
fn active_test_and_ignored_sources_are_distinct() {
    let bytes=br#"{"Dir":"/project","ImportPath":"example.com/app","Name":"main","GoFiles":["main.go"],"TestGoFiles":["main_test.go"],"XTestGoFiles":["external_test.go"],"IgnoredGoFiles":["tagged.go"]}"#;
    let scope = parse_go_list_scope(Path::new("/project"), bytes).unwrap();
    assert!(scope.active_files.contains("main.go"));
    assert!(scope.active_files.contains("main_test.go"));
    assert!(scope.active_files.contains("external_test.go"));
    assert!(!scope.active_files.contains("tagged.go"));
    assert!(scope.ignored_files.contains("tagged.go"));
}
#[test]
fn invalid_or_conflicting_native_scopes_do_not_become_empty_success() {
    for bytes in [
        "",
        "{}",
        "{\"Dir\":\"/outside\",\"ImportPath\":\"x\",\"Name\":\"x\",\"GoFiles\":[\"a.go\"]}",
        "{\"Dir\":\"/project\",\"ImportPath\":\"x\",\"Name\":\"x\",\"GoFiles\":[\"../a.go\"]}",
        "{\"Dir\":\"/project\",\"ImportPath\":\"x\",\"Name\":\"x\",\"GoFiles\":[\"a.go\"],\"IgnoredGoFiles\":[\"a.go\"]}",
        "{\"Dir\":\"/project\",\"ImportPath\":\"x\",\"Name\":\"x\",\"GoFiles\":[\"a.go\"],\"Error\":{\"Err\":\"broken\"}}",
        "{\"Dir\":\"/project\",\"Dir\":\"/outside\",\"ImportPath\":\"x\",\"Name\":\"x\"}",
    ] {
        assert!(
            parse_go_list_scope(Path::new("/project"), bytes.as_bytes()).is_err(),
            "{bytes}"
        );
    }
}
