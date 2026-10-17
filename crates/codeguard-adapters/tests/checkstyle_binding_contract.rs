use codeguard_adapters::checkstyle_comment_rule_bindings;
const DTD: &str = "<!DOCTYPE module PUBLIC \"-//Checkstyle//DTD Checkstyle Configuration 1.3//EN\" \"https://checkstyle.org/dtds/configuration_1_3.dtd\">";
#[test]
fn custom_sources_bind_only_to_their_original_configured_classes() {
    let bytes = format!(
        "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"JavadocType\"><property name=\"id\" value=\"componentDocs\"/></module><module name=\"JavadocMethod\"/></module></module>"
    );
    let bindings = checkstyle_comment_rule_bindings(bytes.as_bytes(), "10.21.4").unwrap();
    assert_eq!(
        bindings["componentDocs"].checker_class,
        "com.puppycrawl.tools.checkstyle.checks.javadoc.JavadocTypeCheck"
    );
    assert!(!bindings.contains_key("JavadocType"));
    assert!(
        bindings.contains_key("com.puppycrawl.tools.checkstyle.checks.javadoc.JavadocMethodCheck")
    );
    assert!(!bindings["componentDocs"].repair_steps.is_empty());
    assert!(checkstyle_comment_rule_bindings(bytes.as_bytes(), "14.1.0").is_none());
}
#[test]
fn ambiguous_bindings_are_not_resolved_by_order() {
    for names in [
        ["JavadocType", "JavadocMethod"],
        ["JavadocMethod", "JavadocType"],
        ["JavadocStyle", "SummaryJavadoc"],
        ["NonEmptyAtclauseDescription", "JavadocMethod"],
    ] {
        let body = names
            .map(|n| {
                format!("<module name=\"{n}\"><property name=\"id\" value=\"shared\"/></module>")
            })
            .join("");
        let bytes = format!(
            "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\">{body}</module></module>"
        );
        assert!(checkstyle_comment_rule_bindings(bytes.as_bytes(), "10.21.4").is_none());
    }
}

#[test]
fn field_javadoc_rule_keeps_original_custom_identity_and_repair_direction() {
    let bytes = format!(
        "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"JavadocVariable\"><property name=\"id\" value=\"fieldDocs\"/><property name=\"scope\" value=\"public\"/></module></module></module>"
    );
    let bindings = checkstyle_comment_rule_bindings(bytes.as_bytes(), "10.21.4")
        .expect("native field rule should bind");
    assert_eq!(
        bindings["fieldDocs"].checker_class,
        "com.puppycrawl.tools.checkstyle.checks.javadoc.JavadocVariableCheck"
    );
    assert!(bindings["fieldDocs"].repair_steps[0].contains("字段"));
    assert!(!bindings.contains_key("JavadocVariable"));
    let ambiguous = bytes.replace("</module></module></module>", "</module><module name=\"MissingJavadocType\"><property name=\"id\" value=\"fieldDocs\"/></module></module></module>");
    assert!(checkstyle_comment_rule_bindings(ambiguous.as_bytes(), "10.21.4").is_none());
}

#[test]
fn field_tokens_are_native_configuration_and_cannot_escape_module_context() {
    let bytes = format!(
        "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"JavadocVariable\"><property name=\"tokens\" value=\"VARIABLE_DEF, ENUM_CONSTANT_DEF\"/></module></module></module>"
    );
    assert!(checkstyle_comment_rule_bindings(bytes.as_bytes(), "10.21.4").is_some());
    for invalid in [
        bytes.replace("VARIABLE_DEF, ENUM_CONSTANT_DEF", "METHOD_DEF"),
        bytes.replace("JavadocVariable", "MissingJavadocType"),
    ] {
        assert!(checkstyle_comment_rule_bindings(invalid.as_bytes(), "10.21.4").is_none());
    }
}

#[test]
fn official_full_names_bind_without_suffix_guessing_or_duplicate_identity() {
    for name in [
        "MissingJavadocType",
        "MissingJavadocMethod",
        "JavadocType",
        "JavadocMethod",
        "JavadocVariable",
        "JavadocStyle",
        "NonEmptyAtclauseDescription",
        "SummaryJavadoc",
    ] {
        let class = format!("com.puppycrawl.tools.checkstyle.checks.javadoc.{name}Check");
        let full = format!(
            "{DTD}<module name=\"com.puppycrawl.tools.checkstyle.Checker\"><module name=\"com.puppycrawl.tools.checkstyle.TreeWalker\"><module name=\"{class}\"/></module></module>"
        );
        let binding = checkstyle_comment_rule_bindings(full.as_bytes(), "10.21.4")
            .expect("official full name must remain native configuration");
        assert_eq!(binding[&class].checker_class, class);
        for alias in [name.to_owned(), format!("{name}Check")] {
            let alias_config =
                full.replace(&format!("name=\"{class}\""), &format!("name=\"{alias}\""));
            assert_eq!(
                checkstyle_comment_rule_bindings(alias_config.as_bytes(), "10.21.4").unwrap(),
                binding
            );
        }

        let duplicate = full.replace(
            &format!("<module name=\"{class}\"/>"),
            &format!("<module name=\"{class}\"/><module name=\"{name}\"/>"),
        );
        assert!(checkstyle_comment_rule_bindings(duplicate.as_bytes(), "10.21.4").is_none());
        assert!(
            checkstyle_comment_rule_bindings(
                full.replace(
                    "com.puppycrawl.tools.checkstyle.checks.javadoc.",
                    "untrusted.checks."
                )
                .as_bytes(),
                "10.21.4"
            )
            .is_none()
        );
    }
}

#[test]
fn missing_method_configuration_retains_native_exclusions_and_limits() {
    let full = "com.puppycrawl.tools.checkstyle.checks.javadoc.MissingJavadocMethodCheck";
    let body = "<property name=\"scope\" value=\"public\"/><property name=\"excludeScope\" value=\"protected\"/><property name=\"allowedAnnotations\" value=\"SkipDocs, example.Other\"/><property name=\"allowMissingPropertyJavadoc\" value=\"true\"/><property name=\"minLineCount\" value=\"2\"/><property name=\"ignoreMethodNamesRegex\" value=\"ignored.*\"/>";
    let xml = format!(
        "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"{full}\">{body}</module></module></module>"
    );
    assert!(checkstyle_comment_rule_bindings(xml.as_bytes(), "10.21.4").is_some());
    for invalid in [
        xml.replace(
            full,
            "com.puppycrawl.tools.checkstyle.checks.javadoc.JavadocVariableCheck",
        ),
        xml.replace("value=\"true\"", "value=\"maybe\""),
        xml.replace("value=\"2\"", "value=\"NaN\""),
    ] {
        assert!(checkstyle_comment_rule_bindings(invalid.as_bytes(), "10.21.4").is_none());
    }
}

#[test]
fn method_tokens_bind_only_to_native_method_and_constructor_context() {
    let xml = format!(
        "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"MissingJavadocMethod\"><property name=\"tokens\" value=\"METHOD_DEF, CTOR_DEF, COMPACT_CTOR_DEF, ANNOTATION_FIELD_DEF\"/></module></module></module>"
    );
    assert!(checkstyle_comment_rule_bindings(xml.as_bytes(), "10.21.4").is_some());
    for invalid in [
        xml.replace(
            "METHOD_DEF, CTOR_DEF, COMPACT_CTOR_DEF, ANNOTATION_FIELD_DEF",
            "VARIABLE_DEF",
        ),
        xml.replace("MissingJavadocMethod", "JavadocVariable"),
    ] {
        assert!(checkstyle_comment_rule_bindings(invalid.as_bytes(), "10.21.4").is_none());
    }
}

#[test]
fn method_tag_configuration_retains_native_options_and_context() {
    let xml = format!(
        "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"JavadocMethod\"><property name=\"accessModifiers\" value=\"public,protected\"/><property name=\"allowMissingParamTags\" value=\"true\"/><property name=\"allowMissingReturnTag\" value=\"false\"/><property name=\"validateThrows\" value=\"true\"/><property name=\"allowedAnnotations\" value=\"SkipDocs\"/><property name=\"tokens\" value=\"METHOD_DEF, CTOR_DEF, ANNOTATION_FIELD_DEF, COMPACT_CTOR_DEF\"/></module></module></module>"
    );
    assert!(checkstyle_comment_rule_bindings(xml.as_bytes(), "10.21.4").is_some());
    for invalid in [
        xml.replace("JavadocMethod", "MissingJavadocMethod"),
        xml.replace("public,protected", "public,internal"),
        xml.replace("value=\"true\"", "value=\"maybe\""),
        xml.replace(
            "METHOD_DEF, CTOR_DEF, ANNOTATION_FIELD_DEF, COMPACT_CTOR_DEF",
            "VARIABLE_DEF",
        ),
    ] {
        assert!(checkstyle_comment_rule_bindings(invalid.as_bytes(), "10.21.4").is_none());
    }
}

#[test]
fn type_documentation_options_remain_in_their_native_modules() {
    for (module, properties) in [
        (
            "MissingJavadocType",
            "<property name=\"scope\" value=\"public\"/><property name=\"excludeScope\" value=\"protected\"/><property name=\"skipAnnotations\" value=\"SkipDocs, example.Other\"/>",
        ),
        (
            "JavadocType",
            "<property name=\"scope\" value=\"private\"/><property name=\"allowMissingParamTags\" value=\"true\"/><property name=\"allowUnknownTags\" value=\"true\"/><property name=\"allowedAnnotations\" value=\"SkipDocs\"/><property name=\"authorFormat\" value=\".+\"/><property name=\"versionFormat\" value=\"[0-9]+\"/>",
        ),
    ] {
        let xml = format!(
            "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"{module}\">{properties}<property name=\"tokens\" value=\"CLASS_DEF, INTERFACE_DEF, ENUM_DEF, ANNOTATION_DEF, RECORD_DEF\"/></module></module></module>"
        );
        assert!(
            checkstyle_comment_rule_bindings(xml.as_bytes(), "10.21.4").is_some(),
            "{module}"
        );
        for invalid in [
            xml.replace(module, "JavadocMethod"),
            xml.replace(
                "CLASS_DEF, INTERFACE_DEF, ENUM_DEF, ANNOTATION_DEF, RECORD_DEF",
                "METHOD_DEF",
            ),
        ] {
            assert!(checkstyle_comment_rule_bindings(invalid.as_bytes(), "10.21.4").is_none());
        }
    }
}

#[test]
fn type_tag_repair_guidance_covers_author_and_version_without_inventing_values() {
    let xml = format!(
        "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"JavadocType\"><property name=\"id\" value=\"typeTags\"/><property name=\"authorFormat\" value=\".+\"/><property name=\"versionFormat\" value=\".+\"/></module></module></module>"
    );
    let bindings = checkstyle_comment_rule_bindings(xml.as_bytes(), "10.21.4").unwrap();
    let step = bindings["typeTags"].repair_steps[0];
    assert!(step.contains("@author") && step.contains("@version"));
    assert!(step.contains("不编造"));
}

#[test]
fn detailed_documentation_modules_preserve_original_properties_and_custom_sources() {
    for (name, properties, keyword) in [
        (
            "JavadocStyle",
            "<property name=\"checkEmptyJavadoc\" value=\"true\"/><property name=\"checkFirstSentence\" value=\"false\"/><property name=\"checkHtml\" value=\"true\"/><property name=\"scope\" value=\"public\"/><property name=\"tokens\" value=\"RECORD_DEF, COMPACT_CTOR_DEF, METHOD_DEF, VARIABLE_DEF\"/><property name=\"endOfSentenceFormat\" value=\"[。.!?]$\"/>",
            "用途",
        ),
        (
            "NonEmptyAtclauseDescription",
            "<property name=\"javadocTokens\" value=\"PARAM_LITERAL, RETURN_LITERAL, THROWS_LITERAL, EXCEPTION_LITERAL, DEPRECATED_LITERAL\"/><property name=\"violateExecutionOnNonTightHtml\" value=\"true\"/>",
            "参数",
        ),
        (
            "SummaryJavadoc",
            "<property name=\"period\" value=\"。\"/><property name=\"forbiddenSummaryFragments\" value=\"^(TODO|待补充)$\"/><property name=\"violateExecutionOnNonTightHtml\" value=\"false\"/>",
            "摘要",
        ),
    ] {
        let xml = format!(
            "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"{name}\"><property name=\"id\" value=\"detailedDocs\"/>{properties}</module></module></module>"
        );
        let map = checkstyle_comment_rule_bindings(xml.as_bytes(), "10.21.4")
            .expect("official detailed documentation config");
        assert_eq!(
            map["detailedDocs"].checker_class,
            format!("com.puppycrawl.tools.checkstyle.checks.javadoc.{name}Check")
        );
        assert!(map["detailedDocs"].repair_steps[0].contains(keyword));
        assert!(
            checkstyle_comment_rule_bindings(
                xml.replace("name=\"TreeWalker\"", "name=\"UnknownWalker\"")
                    .as_bytes(),
                "10.21.4"
            )
            .is_none()
        );
        assert!(
            checkstyle_comment_rule_bindings(
                xml.replace("value=\"true\"", "value=\"maybe\"")
                    .replace("value=\"false\"", "value=\"maybe\"")
                    .as_bytes(),
                "10.21.4"
            )
            .is_none()
        );
        for alias in [
            format!("{name}Check"),
            format!("com.puppycrawl.tools.checkstyle.checks.javadoc.{name}Check"),
        ] {
            let changed = xml.replace(&format!("name=\"{name}\""), &format!("name=\"{alias}\""));
            assert_eq!(
                checkstyle_comment_rule_bindings(changed.as_bytes(), "10.21.4").unwrap(),
                map
            );
        }
    }
}

#[test]
fn detailed_documentation_properties_do_not_escape_the_native_module_or_token_set() {
    for (name, properties) in [
        (
            "JavadocMethod",
            "<property name=\"checkEmptyJavadoc\" value=\"true\"/>",
        ),
        (
            "SummaryJavadoc",
            "<property name=\"javadocTokens\" value=\"PARAM_LITERAL\"/>",
        ),
        (
            "NonEmptyAtclauseDescription",
            "<property name=\"javadocTokens\" value=\"METHOD_DEF\"/>",
        ),
        (
            "JavadocStyle",
            "<property name=\"tokens\" value=\"PARAM_LITERAL\"/>",
        ),
        (
            "JavadocStyle",
            "<property name=\"checkEmptyJavadoc\" value=\"maybe\"/>",
        ),
        (
            "NonEmptyAtclauseDescription",
            "<property name=\"period\" value=\"。\"/>",
        ),
        (
            "SummaryJavadoc",
            "<property name=\"scope\" value=\"public\"/>",
        ),
    ] {
        let xml = format!(
            "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"{name}\">{properties}</module></module></module>"
        );
        assert!(
            checkstyle_comment_rule_bindings(xml.as_bytes(), "10.21.4").is_none(),
            "{xml}"
        );
    }
}

#[test]
fn summary_native_root_token_and_empty_options_are_preserved_without_rust_regex_evaluation() {
    let xml = format!(
        "{DTD}<module name=\"Checker\"><module name=\"TreeWalker\"><module name=\"SummaryJavadoc\"><property name=\"javadocTokens\" value=\"JAVADOC\"/><property name=\"period\" value=\"\"/><property name=\"forbiddenSummaryFragments\" value=\"\"/></module></module></module>"
    );
    assert!(checkstyle_comment_rule_bindings(xml.as_bytes(), "10.21.4").is_some());
    let invalid_regex = xml.replace(
        "name=\"forbiddenSummaryFragments\" value=\"\"",
        "name=\"forbiddenSummaryFragments\" value=\"[\"",
    );
    assert!(checkstyle_comment_rule_bindings(invalid_regex.as_bytes(), "10.21.4").is_some());
}
