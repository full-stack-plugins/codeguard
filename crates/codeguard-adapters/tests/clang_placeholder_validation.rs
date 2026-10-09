use codeguard_adapters::{
    parse_clang_documentation_ast, parse_clang_documentation_placeholders,
    valid_clang_documentation_placeholders,
};
use serde_json::json;

#[test]
fn imported_placeholder_positions_require_closed_shape_and_matching_supported_components() {
    let source = b"void f(int x);";
    let ast = json!({"kind":"TranslationUnitDecl","inner":[{"kind":"FunctionDecl","name":"f","loc":{"offset":5,"tokLen":1},"type":{"qualType":"void (int)"},"inner":[{"kind":"ParmVarDecl","name":"x"},{"kind":"FullComment","inner":[{"kind":"ParagraphComment","inner":[{"kind":"TextComment","text":"TODO"}]},{"kind":"ParamCommandComment","param":"x","paramIdx":0,"inner":[{"kind":"ParagraphComment","inner":[{"kind":"TextComment","text":"TBD"}]}]}]}]}]});
    let raw = serde_json::to_vec(&ast).unwrap();
    let structure = parse_clang_documentation_ast(&raw, source).unwrap();
    let report = parse_clang_documentation_placeholders(&raw, source).unwrap();
    assert!(valid_clang_documentation_placeholders(
        &report,
        &structure,
        Some(source)
    ));
    assert!(!valid_clang_documentation_placeholders(
        &report,
        &structure,
        Some(b"void g(int x);")
    ));
    for (field, value) in [
        ("component", json!("return")),
        ("component", json!("parameter:y")),
        ("line", json!(2)),
        ("offset_byte", json!(6)),
        ("unexpected", json!(true)),
    ] {
        let mut forged = report.clone();
        forged["positions"][0][field] = value;
        assert!(
            !valid_clang_documentation_placeholders(&forged, &structure, Some(source)),
            "{field}"
        );
    }
    let mut forged = report.clone();
    forged["positions"]
        .as_array_mut()
        .unwrap()
        .push(report["positions"][0].clone());
    assert!(!valid_clang_documentation_placeholders(
        &forged,
        &structure,
        Some(source)
    ));
    for (field, value) in [
        ("qualification", json!("granted")),
        ("authority", json!("native_tool")),
        ("coverage_proven", json!(true)),
        ("unexpected", json!(true)),
    ] {
        let mut forged = report.clone();
        forged[field] = value;
        assert!(!valid_clang_documentation_placeholders(
            &forged,
            &structure,
            Some(source)
        ));
    }
}
