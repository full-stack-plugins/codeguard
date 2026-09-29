#![cfg(feature = "wasm-precheck")]

use codeguard_runtime::WasmGrammar;

const DART: &[u8] = include_bytes!("../../../grammars/dart/parser.wasm");
const CORPORA: &[(&str, &str)] = &[
    (
        "annotations",
        include_str!("../../../grammars/dart/corpus/annotations.txt"),
    ),
    (
        "big_tests",
        include_str!("../../../grammars/dart/corpus/big_tests.txt"),
    ),
    (
        "class_modifiers",
        include_str!("../../../grammars/dart/corpus/class_modifiers.txt"),
    ),
    (
        "comments",
        include_str!("../../../grammars/dart/corpus/comments.txt"),
    ),
    (
        "dart",
        include_str!("../../../grammars/dart/corpus/dart.txt"),
    ),
    (
        "declarations",
        include_str!("../../../grammars/dart/corpus/declarations.txt"),
    ),
    (
        "enhanced_enums",
        include_str!("../../../grammars/dart/corpus/enhanced_enums.txt"),
    ),
    (
        "errors",
        include_str!("../../../grammars/dart/corpus/errors.txt"),
    ),
    (
        "expressions",
        include_str!("../../../grammars/dart/corpus/expressions.txt"),
    ),
    (
        "flutter",
        include_str!("../../../grammars/dart/corpus/flutter.txt"),
    ),
    (
        "literals",
        include_str!("../../../grammars/dart/corpus/literals.txt"),
    ),
    (
        "more_expressions",
        include_str!("../../../grammars/dart/corpus/more_expressions.txt"),
    ),
    (
        "patterns",
        include_str!("../../../grammars/dart/corpus/patterns.txt"),
    ),
    (
        "records",
        include_str!("../../../grammars/dart/corpus/records.txt"),
    ),
    (
        "types",
        include_str!("../../../grammars/dart/corpus/types.txt"),
    ),
];

#[test]
fn rebuilt_dart_preserves_upstream_corpus_error_classification() {
    let mut grammar = WasmGrammar::load(
        "dart",
        DART,
        "7dad281b3b24924d619cb7059a42b409e7690ebeb8ebbc82882e68167656e012",
        15,
    )
    .expect("fixed Dart grammar");
    let mut total = 0usize;
    let mut negative = 0usize;
    let mut mismatches = Vec::new();
    for (file, corpus) in CORPORA {
        let lines = corpus.lines().collect::<Vec<_>>();
        let mut index = 0usize;
        while index + 2 < lines.len() {
            if !divider(lines[index]) || !divider(lines[index + 2]) {
                index += 1;
                continue;
            }
            let title = lines[index + 1];
            index += 3;
            let source_start = index;
            while index < lines.len() && !separator(lines[index]) {
                index += 1;
            }
            assert!(
                index < lines.len(),
                "{file}/{title}: no expectation separator"
            );
            let source = lines[source_start..index].join("\n");
            index += 1;
            let expected_start = index;
            while index + 2 < lines.len() && !(divider(lines[index]) && divider(lines[index + 2])) {
                index += 1;
            }
            let expected = lines[expected_start..index].join("\n");
            let expected_error = expected.contains("(ERROR") || expected.contains("(MISSING");
            let actual_error = grammar
                .parse(source.as_bytes())
                .unwrap_or_else(|reason| panic!("{file}/{title}: {reason}"))
                .root_node()
                .has_error();
            total += 1;
            negative += usize::from(expected_error);
            if actual_error != expected_error {
                mismatches.push(format!(
                    "{file}/{title}: expected error={expected_error}, actual={actual_error}"
                ));
            }
        }
    }
    assert_eq!(total, 150, "上游 corpus 用例数量变化");
    assert_eq!(negative, 4, "上游错误样例数量变化");
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

fn divider(line: &str) -> bool {
    line.len() >= 4 && line.bytes().all(|byte| byte == b'=')
}

fn separator(line: &str) -> bool {
    line.len() >= 3 && line.bytes().all(|byte| byte == b'-')
}
