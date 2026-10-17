use codeguard_adapters::parse_clang_documentation_placeholders;
use serde_json::json;

#[test]
fn exact_markers_are_deficits_but_mentions_are_not_and_void_has_no_return_requirement() {
    for (text, expected) in [
        (" TODO. ", true),
        ("TBD", true),
        ("FIXME!", true),
        ("待补充。", true),
        ("TODO queue behavior is documented here.", false),
        ("Returns the input value.", false),
    ] {
        let ast = json!({"kind":"TranslationUnitDecl","inner":[{"kind":"FunctionDecl","name":"f","loc":{"offset":5,"tokLen":1},"type":{"qualType":"void (int)"},"inner":[{"kind":"ParmVarDecl","name":"x"},{"kind":"FullComment","inner":[{"kind":"ParagraphComment","inner":[{"kind":"TextComment","text":text}]},{"kind":"ParamCommandComment","param":"x","paramIdx":0,"inner":[{"kind":"ParagraphComment","inner":[{"kind":"TextComment","text":text}]}]}]}]}]});
        let observed = parse_clang_documentation_placeholders(
            &serde_json::to_vec(&ast).unwrap(),
            b"void f(int x);",
        )
        .unwrap();
        assert_eq!(
            observed["positions"].as_array().unwrap().len(),
            if expected { 2 } else { 0 },
            "{text}"
        );
        assert_eq!(observed["qualification"], "not_granted");
        assert_eq!(observed["coverage_proven"], false);
    }
}

#[test]
#[ignore = "requires explicit existing Clang via CODEGUARD_CLANG_BIN"]
fn actual_clang_placeholder_tags_and_mentions_keep_separate_authority() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let tool = std::env::var("CODEGUARD_CLANG_BIN").unwrap();
    let source = b"/// TODO.\n/// @param x TBD\n/// @return FIXME!\nint f(int x);\n/// TODO queue behavior is explained.\n/// @param x The value to enqueue.\nvoid g(int x);\n";
    let mut child = Command::new(tool)
        .args([
            "-fsyntax-only",
            "-x",
            "c",
            "-std=c11",
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
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report = parse_clang_documentation_placeholders(&result.stdout, source).unwrap();
    let positions = report["positions"].as_array().unwrap();
    assert_eq!(positions.len(), 3);
    for component in ["purpose", "parameter:x", "return"] {
        assert!(
            positions
                .iter()
                .any(|position| position["component"] == component && position["line"] == 4)
        );
    }
    assert_eq!(report["authority"], "codeguard_structural_policy");
    assert_eq!(report["coverage_proven"], false);
    if let Ok(destination) = std::env::var("CODEGUARD_PLACEHOLDER_EVIDENCE") {
        let path = std::path::Path::new(&destination);
        assert!(path.is_absolute());
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}
