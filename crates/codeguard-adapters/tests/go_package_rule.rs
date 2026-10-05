use codeguard_adapters::{go_package_rule_sha256, missing_go_package_candidate};
use sha2::{Digest, Sha256};

#[test]
fn only_complete_go_whole_file_facts_establish_a_candidate() {
    assert_eq!(
        missing_go_package_candidate("go", true, "source_file", "package_clause", false, false),
        Some(true)
    );
    assert_eq!(
        missing_go_package_candidate("go", true, "source_file", "package_clause", true, false),
        Some(false)
    );
    for (language, whole_file, root, present, truncated) in [
        ("go", false, "source_file", false, false),
        ("rust", true, "source_file", false, false),
        ("go", true, "unknown_root", false, false),
        ("go", true, "source_file", false, true),
        ("go", true, "source_file", true, true),
    ] {
        assert_eq!(
            missing_go_package_candidate(
                language,
                whole_file,
                root,
                "package_clause",
                present,
                truncated
            ),
            None
        );
    }
    assert_eq!(
        missing_go_package_candidate(
            "go",
            true,
            "source_file",
            "function_declaration",
            false,
            false
        ),
        None
    );
    assert_eq!(
        go_package_rule_sha256(),
        format!(
            "{:x}",
            Sha256::digest(include_bytes!(
                "../../../rulepacks/go/required_package.json"
            ))
        )
    );
}
