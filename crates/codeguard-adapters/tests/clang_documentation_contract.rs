use codeguard_adapters::clang_documentation_guidance;

#[test]
fn only_precise_native_rules_receive_documentation_repair_guidance() {
    for rule in [
        "clang.warn_doc_block_command_empty_paragraph",
        "clang.warn_doc_param_not_found",
        "clang.warn_doc_returns_attached_to_a_void_function",
    ] {
        let guidance = clang_documentation_guidance(rule).unwrap();
        assert_eq!(guidance["allowed_changes"], "documentation_comment_only");
        assert_eq!(
            guidance["closing_condition"],
            "original_native_recheck_and_approved_complete_coverage_required"
        );
        assert!(guidance["repair_steps"].as_array().unwrap().len() >= 2);
    }
    for rule in [
        "warn_doc_param_not_found",
        "clang.warn_doc_new_rule",
        "clang.warn_unused_variable",
        "clang.warn_doc_param_not_found_extra",
        "clippy::missing_errors_doc",
    ] {
        assert!(clang_documentation_guidance(rule).is_none(), "{rule}");
    }
}
