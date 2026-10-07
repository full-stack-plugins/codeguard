use codeguard_adapters::parse_clang_documentation_ast;
use serde_json::{Value, json};

fn function(source: &str, name: &str, comments: Vec<Value>) -> Value {
    json!({"kind":"TranslationUnitDecl","inner":[{"kind":"FunctionDecl","name":name,
    "loc":{"offset":source.find(name).unwrap(),"tokLen":name.len(),"file":"<stdin>"},
    "type":{"qualType":"int (int)"},"inner":[
        {"kind":"ParmVarDecl","name":"x"},
        {"kind":"FullComment","inner":comments}
    ]}]})
}

#[test]
fn zero_native_warnings_do_not_hide_missing_or_empty_documentation() {
    let source = "int f(int x);";
    let mut ast = function(source, "f", vec![]);
    ast["inner"][0]["inner"].as_array_mut().unwrap().pop();
    let r = parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source.as_bytes())
        .unwrap();
    assert_eq!(r["functions"][0]["comment_presence"], "absent");
    assert_eq!(
        r["functions"][0]["missing_components"],
        json!(["documentation_comment"])
    );
    let r = parse_clang_documentation_ast(
        &serde_json::to_vec(&function(source, "f", vec![])).unwrap(),
        source.as_bytes(),
    )
    .unwrap();
    assert_eq!(r["functions"][0]["comment_presence"], "present");
    assert_eq!(
        r["functions"][0]["missing_components"],
        json!(["purpose", "parameter:x", "return"])
    );
    assert_eq!(r["qualification"], "not_granted");
}

#[test]
fn descriptions_are_attached_to_the_actual_parameter_and_return_commands() {
    let source = "int f(int x);";
    let paragraph =
        |text| json!({"kind":"ParagraphComment","inner":[{"kind":"TextComment","text":text}]});
    let ast = function(
        source,
        "f",
        vec![
            paragraph("计算输入的平方。"),
            json!({"kind":"ParamCommandComment","param":"x","paramIdx":0,"inner":[paragraph("输入整数。")]}),
            json!({"kind":"BlockCommandComment","name":"return","inner":[paragraph("计算结果。")]}),
        ],
    );
    let r = parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source.as_bytes())
        .unwrap();
    assert_eq!(r["functions"][0]["missing_components"], json!([]));
    assert_eq!(r["functions"][0]["return_description"], "nonempty");
    let mut wrong = ast.clone();
    wrong["inner"][0]["inner"][1]["inner"][1]["param"] = json!("another");
    assert!(
        parse_clang_documentation_ast(&serde_json::to_vec(&wrong).unwrap(), source.as_bytes())
            .is_err()
    );
}

#[test]
fn inherited_comments_complex_declarations_and_mismatched_source_stay_untrusted() {
    let source = "int f(int x);";
    let mut ast = function(source, "f", vec![]);
    ast["inner"][0]["previousDecl"] = json!("0x123");
    let r = parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source.as_bytes())
        .unwrap();
    assert_eq!(
        r["functions"][0]["comment_presence"],
        "unknown_redeclaration"
    );
    assert_eq!(r["functions"][0]["missing_components"], json!([]));
    ast["inner"][0]["kind"] = json!("CXXMethodDecl");
    let r = parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source.as_bytes())
        .unwrap();
    assert_eq!(r["functions"], json!([]));
    assert_eq!(r["unresolved_declaration_kinds"], json!(["CXXMethodDecl"]));
    assert!(
        parse_clang_documentation_ast(
            &serde_json::to_vec(&function(source, "f", vec![])).unwrap(),
            b"int g(int x);"
        )
        .is_err()
    );
    assert!(parse_clang_documentation_ast(b"{}", source.as_bytes()).is_err());
}

#[test]
fn copy_commands_and_unimplemented_markup_do_not_generate_missing_description_claims() {
    let source = "int f(int x);";
    let ast = function(
        source,
        "f",
        vec![
            json!({"kind":"ParagraphComment","inner":[{"kind":"InlineCommandComment","name":"copydoc","args":["original"]}]}),
        ],
    );
    let r = parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source.as_bytes())
        .unwrap();
    assert_eq!(r["functions"][0]["purpose_description"], "unknown");
    assert_eq!(r["functions"][0]["missing_components"], json!([]));
    assert_eq!(
        r["functions"][0]["structure_status"],
        "unresolved_comment_structure"
    );
}

#[test]
fn native_newline_forms_preserve_byte_positions() {
    for newline in ["\n", "\r", "\r\n", "\n\r"] {
        let source = format!("/* 前缀 */{newline}int f(int x);");
        let ast = function(&source, "f", vec![]);
        let r =
            parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source.as_bytes())
                .unwrap();
        assert_eq!(
            r["functions"][0]["line"],
            if newline == "\n\r" { 3 } else { 2 }
        );
        assert_eq!(r["functions"][0]["column_byte"], 5);
    }
}

#[test]
fn oversized_inputs_macro_locations_and_malformed_child_lists_are_rejected() {
    let source = "int f(int x);";
    assert!(
        parse_clang_documentation_ast(&vec![b' '; 8 * 1024 * 1024 + 1], source.as_bytes()).is_err()
    );
    assert!(parse_clang_documentation_ast(b"{}", &vec![b' '; 1024 * 1024 + 1]).is_err());
    let mut ast = function(source, "f", vec![]);
    ast["inner"][0]["loc"]["spellingLoc"] = json!({"offset":4});
    assert!(
        parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source.as_bytes())
            .is_err()
    );
    ast = function(source, "f", vec![]);
    ast["inner"][0]["inner"] = json!({});
    assert!(
        parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source.as_bytes())
            .is_err()
    );
}

#[test]
fn parser_stops_at_report_function_and_parameter_limits() {
    let mut source = String::new();
    let mut declarations = Vec::new();
    for index in 0..2001 {
        let name = format!("f{index}");
        let offset = source.len() + 4;
        source.push_str(&format!("int {name}();\n"));
        declarations.push(json!({"kind":"FunctionDecl","name":name,"loc":{"offset":offset,"tokLen":name.len()},"type":{"qualType":"int ()"}}));
    }
    let mut ast = json!({"kind":"TranslationUnitDecl","inner":declarations});
    assert_eq!(
        parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source.as_bytes()).err(),
        Some("clang_documentation_ast_budget_exceeded")
    );
    ast["inner"].as_array_mut().unwrap().pop();
    assert_eq!(
        parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source.as_bytes())
            .unwrap()["functions"]
            .as_array()
            .unwrap()
            .len(),
        2000
    );
    let source = "int f(int x);";
    let mut ast = function(source, "f", vec![]);
    let params: Vec<_> = (0..257)
        .map(|index| json!({"kind":"ParmVarDecl","name":format!("x{index}")}))
        .collect();
    ast["inner"][0]["inner"] = json!(params);
    assert_eq!(
        parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source.as_bytes()).err(),
        Some("clang_documentation_ast_budget_exceeded")
    );
    ast["inner"][0]["inner"].as_array_mut().unwrap().pop();
    assert!(
        parse_clang_documentation_ast(&serde_json::to_vec(&ast).unwrap(), source.as_bytes())
            .is_ok()
    );
}
