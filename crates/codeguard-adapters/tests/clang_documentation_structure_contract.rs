use codeguard_adapters::{parse_clang_documentation_ast, valid_clang_documentation_structure};
use serde_json::json;

#[test]
fn structural_import_requires_exact_components_and_actual_source_positions() {
    let source = b"int f(int x);";
    let ast = json!({"kind":"TranslationUnitDecl","inner":[{"kind":"FunctionDecl","name":"f","loc":{"offset":4,"tokLen":1,"file":"<stdin>"},"type":{"qualType":"int (int)"},"inner":[{"kind":"ParmVarDecl","name":"x"}]}]});
    let observation =
        parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source).unwrap();
    assert!(valid_clang_documentation_structure(
        &observation,
        Some(source)
    ));
    assert!(!valid_clang_documentation_structure(
        &observation,
        Some(b"int g(int x);")
    ));
    for (field, value) in [
        ("missing_components", json!([])),
        ("line", json!(2)),
        ("comment_presence", json!("present")),
        ("unexpected", json!(true)),
    ] {
        let mut forged = observation.clone();
        forged["functions"][0][field] = value;
        assert!(
            !valid_clang_documentation_structure(&forged, Some(source)),
            "{field}"
        );
    }
    let mut forged = observation.clone();
    forged["functions"]
        .as_array_mut()
        .unwrap()
        .push(observation["functions"][0].clone());
    assert!(!valid_clang_documentation_structure(&forged, None));
    let mut forged = observation;
    forged["qualification"] = json!("granted");
    assert!(!valid_clang_documentation_structure(&forged, None));
}
