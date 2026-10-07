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

#[test]
fn ordinary_cpp_class_methods_are_observed_without_claiming_class_contracts() {
    let source = b"struct Box { int read(int x); };";
    let offset = source.windows(4).position(|w| w == b"read").unwrap();
    let ast = json!({"kind":"TranslationUnitDecl","inner":[{"kind":"CXXRecordDecl","name":"Box","inner":[
        {"kind":"CXXRecordDecl","name":"Box","isImplicit":true},
        {"kind":"CXXMethodDecl","name":"read","loc":{"offset":offset,"tokLen":4},"type":{"qualType":"int (int)"},"inner":[{"kind":"ParmVarDecl","name":"x"}]}
    ]}]});
    let observed =
        parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source).unwrap();
    assert_eq!(observed["functions"].as_array().unwrap().len(), 1);
    assert_eq!(observed["functions"][0]["name"], "read");
    assert_eq!(
        observed["functions"][0]["missing_components"],
        json!(["documentation_comment"])
    );
    assert!(
        observed["unresolved_declaration_kinds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "CXXRecordDecl")
    );
    assert!(valid_clang_documentation_structure(&observed, Some(source)));
    assert_eq!(observed["coverage_proven"], false);
}

#[test]
#[ignore = "requires explicit existing Apple Clang21 via CODEGUARD_CLANG_BIN"]
fn actual_cpp17_ast_retains_method_descriptions_and_unsupported_contracts() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let clang = std::env::var("CODEGUARD_CLANG_BIN").expect("explicit existing Clang");
    let version = Command::new(&clang).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert!(
        String::from_utf8_lossy(&version.stdout)
            .starts_with("Apple clang version 21.0.0 (clang-2100.3.34.2)")
    );
    let source = b"struct Box {\n/// Read a value.\n/// @param x Input value.\n/// @return The value.\nint read(int x);\n/// Clear the box.\nvoid clear();\nint undocumented();\n/// Create the box.\n/// @param size Initial size.\nBox(int size);\nint operator+(int x);\n};\n";
    let mut child = Command::new(&clang)
        .args([
            "-fsyntax-only",
            "-x",
            "c++",
            "-std=c++17",
            "-fparse-all-comments",
            "-Xclang",
            "-ast-dump=json",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(source).unwrap();
    let native = child.wait_with_output().unwrap();
    assert!(
        native.status.success(),
        "{}",
        String::from_utf8_lossy(&native.stderr)
    );
    let observed = parse_clang_documentation_ast(&native.stdout, source).unwrap();
    assert!(
        valid_clang_documentation_structure(&observed, Some(source)),
        "{observed}"
    );
    let functions = observed["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 4, "{observed}");
    for name in ["read", "clear", "Box"] {
        let method = functions.iter().find(|m| m["name"] == name).unwrap();
        assert_eq!(method["missing_components"], json!([]));
    }
    let undocumented = functions
        .iter()
        .find(|m| m["name"] == "undocumented")
        .unwrap();
    assert_eq!(
        undocumented["missing_components"],
        json!(["documentation_comment"])
    );
    for kind in ["CXXRecordDecl", "CXXMethodDecl"] {
        assert!(
            observed["unresolved_declaration_kinds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == kind)
        );
    }
    assert_eq!(observed["qualification"], "not_granted");
    if let Ok(destination) = std::env::var("CODEGUARD_CPP_METHOD_EVIDENCE") {
        use sha2::{Digest, Sha256};
        let path = std::path::Path::new(&destination);
        assert!(path.is_absolute());
        let evidence = json!({
            "qualification":"not_granted",
            "scope":"cpp17_ordinary_in_class_methods_and_explicit_constructors",
            "source_sha256":format!("{:x}", Sha256::digest(source)),
            "test_source_sha256":format!("{:x}", Sha256::digest(include_bytes!("clang_documentation_structure_contract.rs"))),
            "compiler_version":String::from_utf8_lossy(&version.stdout).trim(),
            "compiler_sha256":format!("{:x}", Sha256::digest(std::fs::read(&clang).unwrap())),
            "native_ast_sha256":format!("{:x}", Sha256::digest(&native.stdout)),
            "observation":observed
        });
        std::fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}

#[test]
fn explicit_constructor_requires_parameters_but_never_invents_return_contract() {
    let source = b"struct Box { Box(int size); };";
    let offset = source.windows(3).rposition(|w| w == b"Box").unwrap();
    let constructor = json!({"kind":"CXXConstructorDecl","name":"Box","loc":{"offset":offset,"tokLen":3},"type":{"qualType":"void (int)"},"inner":[
        {"kind":"ParmVarDecl","name":"size"},
        {"kind":"FullComment","inner":[
            {"kind":"ParagraphComment","inner":[{"kind":"TextComment","text":"Create the box."}]},
            {"kind":"ParamCommandComment","param":"size","paramIdx":0,"inner":[{"kind":"ParagraphComment","inner":[{"kind":"TextComment","text":"Initial size."}]}]}
        ]}
    ]});
    let mut ast = json!({"kind":"TranslationUnitDecl","inner":[{"kind":"CXXRecordDecl","name":"Box","inner":[constructor.clone()]}]});
    let observed =
        parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source).unwrap();
    assert_eq!(observed["functions"].as_array().unwrap().len(), 1);
    assert_eq!(
        observed["functions"][0]["return_description"],
        "not_applicable"
    );
    assert_eq!(observed["functions"][0]["missing_components"], json!([]));
    assert!(valid_clang_documentation_structure(&observed, Some(source)));
    ast["inner"][0]["inner"][0]["inner"][1]["inner"][1]["inner"][0]["inner"][0]["text"] =
        json!(" ");
    let observed =
        parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source).unwrap();
    assert_eq!(
        observed["functions"][0]["missing_components"],
        json!(["parameter:size"])
    );
    let outside = json!({"kind":"TranslationUnitDecl","inner":[constructor]});
    let observed =
        parse_clang_documentation_ast(&serde_json::to_vec(&outside).unwrap(), source).unwrap();
    assert_eq!(observed["functions"], json!([]));
    assert_eq!(
        observed["unresolved_declaration_kinds"],
        json!(["CXXConstructorDecl"])
    );
}

#[test]
#[ignore = "requires explicit existing Apple Clang21 via CODEGUARD_CLANG_BIN"]
fn actual_free_operator_does_not_erase_ordinary_identifier_documentation() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let clang = std::env::var("CODEGUARD_CLANG_BIN").expect("explicit existing Clang");
    let version = Command::new(&clang).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert!(
        String::from_utf8_lossy(&version.stdout)
            .starts_with("Apple clang version 21.0.0 (clang-2100.3.34.2)")
    );
    let source =
        b"struct Box {}; Box operator+(Box a, Box b); int ordinary(); int operator_helper();";
    let mut child = Command::new(&clang)
        .args([
            "-fsyntax-only",
            "-x",
            "c++",
            "-std=c++17",
            "-fparse-all-comments",
            "-Xclang",
            "-ast-dump=json",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(source).unwrap();
    let native = child.wait_with_output().unwrap();
    assert!(
        native.status.success(),
        "{}",
        String::from_utf8_lossy(&native.stderr)
    );
    let observed = parse_clang_documentation_ast(&native.stdout, source).unwrap();
    assert!(
        valid_clang_documentation_structure(&observed, Some(source)),
        "{observed}"
    );
    let functions = observed["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 2);
    for name in ["ordinary", "operator_helper"] {
        assert!(functions.iter().any(|f| f["name"] == name));
    }
    assert!(
        observed["unresolved_declaration_kinds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|kind| kind == "FunctionDecl")
    );
    assert_eq!(observed["coverage_proven"], false);
    if let Ok(destination) = std::env::var("CODEGUARD_CPP_FREE_OPERATOR_EVIDENCE") {
        use sha2::{Digest, Sha256};
        let path = std::path::Path::new(&destination);
        assert!(path.is_absolute());
        let evidence = json!({
            "qualification":"not_granted",
            "scope":"unsupported_free_operator_preserves_ordinary_functions",
            "source_sha256":format!("{:x}", Sha256::digest(source)),
            "test_source_sha256":format!("{:x}", Sha256::digest(include_bytes!("clang_documentation_structure_contract.rs"))),
            "compiler_version":String::from_utf8_lossy(&version.stdout).trim(),
            "compiler_sha256":format!("{:x}", Sha256::digest(std::fs::read(&clang).unwrap())),
            "native_ast_sha256":format!("{:x}", Sha256::digest(&native.stdout)),
            "observation":observed
        });
        std::fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}
