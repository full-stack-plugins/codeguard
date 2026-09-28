use codeguard_adapters::p3c_rule_in_selected_rulesets;

#[test]
fn native_p3c_diagnostic_must_belong_to_a_selected_ruleset() {
    let naming = ["rulesets/java/ali-naming.xml"];
    assert!(p3c_rule_in_selected_rulesets(
        &naming,
        "AlibabaJavaNaming",
        "ClassNamingShouldBeCamelRule"
    ));
    assert!(!p3c_rule_in_selected_rulesets(
        &naming,
        "AlibabaJavaComments",
        "ClassMustHaveAuthorRule"
    ));
    assert!(!p3c_rule_in_selected_rulesets(
        &naming,
        "AlibabaJavaNaming",
        "ClassMustHaveAuthorRule"
    ));
    assert!(!p3c_rule_in_selected_rulesets(
        &naming,
        "AlibabaJavaNaming",
        "UnknownRule"
    ));
    assert!(p3c_rule_in_selected_rulesets(
        &[
            "rulesets/java/ali-naming.xml",
            "rulesets/java/ali-comment.xml"
        ],
        "AlibabaJavaComments",
        "ClassMustHaveAuthorRule"
    ));
}
