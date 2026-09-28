use codeguard_adapters::checkstyle_comment_config_local_eligible;
const DTD: &str = "<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\">";
#[test]
fn original_static_comment_config_is_eligible_without_default_rules() {
    let xml = format!(
        "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"JavadocMethod\"><property name=\"id\" value=\"publicMethod\"/></module></module></module>"
    );
    assert!(checkstyle_comment_config_local_eligible(xml.as_bytes()));
}
#[test]
fn unknown_resources_modules_entities_and_dynamic_values_require_context() {
    for body in [
        "<module name=\"Checker\"/>",
        "<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"CustomCheck\"/></module></module>",
        "<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"JavadocMethod\"><property name=\"severity\" value=\"${level}\"/></module></module></module>",
        "<module name=\"Checker\"><module name=\"SuppressionFilter\"><property name=\"file\" value=\"remote.xml\"/></module></module>",
    ] {
        assert!(!checkstyle_comment_config_local_eligible(
            format!("{DTD}{body}").as_bytes()
        ));
    }
    assert!(!checkstyle_comment_config_local_eligible(
        b"<!DOCTYPE module SYSTEM 'https://example.invalid/rules.dtd'><module name='Checker'/>"
    ));
    assert!(!checkstyle_comment_config_local_eligible(
        b"<!DOCTYPE module [<!ENTITY e 'x'>]><module name='Checker'/>"
    ));
}

#[test]
fn ambiguous_rule_sources_and_repeated_properties_require_context() {
    for checks in [
        "<module name=\"MissingJavadocType\"><property name=\"id\" value=\"shared\"/></module><module name=\"JavadocMethod\"><property name=\"id\" value=\"shared\"/></module>",
        "<module name=\"JavadocMethod\"/><module name=\"JavadocMethod\"/>",
        "<module name=\"JavadocMethod\"><property name=\"severity\" value=\"error\"/><property name=\"severity\" value=\"warning\"/></module>",
        "<module name=\"JavadocMethod\"><property name=\"id\" value=\"first\"/><property name=\"id\" value=\"last\"/></module>",
    ] {
        let xml = format!(
            "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\">{checks}</module></module>"
        );
        assert!(
            !checkstyle_comment_config_local_eligible(xml.as_bytes()),
            "{checks}"
        );
    }
    let distinct = format!(
        "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"JavadocMethod\"><property name=\"id\" value=\"first\"/></module><module name=\"JavadocMethod\"><property name=\"id\" value=\"second\"/></module></module></module>"
    );
    assert!(checkstyle_comment_config_local_eligible(
        distinct.as_bytes()
    ));
}

#[test]
fn type_parameters_records_and_missing_method_checks_keep_original_modules() {
    let xml = format!(
        "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"JavadocType\"><property name=\"id\" value=\"typeTags\"/></module><module name=\"MissingJavadocMethod\"><property name=\"id\" value=\"missingMethod\"/></module></module></module>"
    );
    assert!(checkstyle_comment_config_local_eligible(xml.as_bytes()));
}
