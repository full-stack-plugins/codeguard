#![cfg(all(feature = "wasm-precheck", unix))]

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn corpus() -> [(&'static str, &'static str, bool); 13] {
    [
        ("plain", "x = 1\n", true),
        ("class", "class Box\n  def value\n    1\n  end\nend\n", true),
        ("block", "[1, 2].map { |x| x + 1 }\n", true),
        ("hash", "h = { name: \"a\", count: 2 }\n", true),
        (
            "interpolation",
            "name = \"world\"\ns = \"hello #{name}\"\n",
            true,
        ),
        ("heredoc", "s = <<~TEXT\n  hello\nTEXT\n", true),
        ("modifier", "puts \"ok\" if true\n", true),
        ("module", "module M\n  CONST = 1\nend\n", true),
        (
            "missing_end",
            "class Box\n  def value\n    1\n  end\n",
            false,
        ),
        ("unclosed_string", "s = \"oops\n", false),
        ("bad_def", "def f(\n  1\nend\n", false),
        ("missing_expr", "x =\n", false),
        ("bad_call", "foo(1, 2\n", false),
    ]
}

fn candidate_is_valid(root: &Path, name: &str, source: &str) -> bool {
    let file = root.join(format!("{name}.rb"));
    fs::write(&file, source).unwrap();
    let candidate = Command::new(env!("CARGO_BIN_EXE_codeguard"))
        .args(["grammar", "probe", "ruby"])
        .arg(&file)
        .arg("--format=json")
        .output()
        .unwrap();
    assert_eq!(candidate.status.code(), Some(3), "{name}: {candidate:?}");
    let report: serde_json::Value = serde_json::from_slice(&candidate.stdout).unwrap();
    assert_eq!(report["grammar_qualified"], false);
    assert_eq!(report["delivery_decision"], "not_evaluated");
    assert_eq!(
        report["reason"],
        serde_json::Value::Null,
        "{name}: {report}"
    );
    report["recoveries"].as_array().unwrap().is_empty()
}

#[test]
fn ruby_worker_preserves_native_labeled_syntax_corpus() {
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("codeguard-ruby-corpus-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    for (name, source, expected_valid) in corpus() {
        assert_eq!(
            candidate_is_valid(&root, name, source),
            expected_valid,
            "{name}: pinned Ruby 2.6.10 syntax classification changed"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires explicit existing Ruby 2.6.10 via CODEGUARD_RUBY_BIN"]
fn pinned_ruby_worker_matches_native_ruby_check_on_syntax_corpus() {
    let ruby = std::env::var("CODEGUARD_RUBY_BIN").expect("provide an existing Ruby executable");
    let version = Command::new(&ruby).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).starts_with("ruby 2.6.10p210 "));
    let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
        "codeguard-ruby-differential-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let mut compared = 0;
    for (name, source, expected_valid) in corpus() {
        let mut native = Command::new(&ruby)
            .args(["-c", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        native
            .stdin
            .take()
            .unwrap()
            .write_all(source.as_bytes())
            .unwrap();
        let native = native.wait_with_output().unwrap();
        assert!(
            matches!(native.status.code(), Some(0 | 1)),
            "{name}: {native:?}"
        );
        assert_eq!(
            native.status.success(),
            expected_valid,
            "{name}: Ruby disagrees with the pinned corpus label: {}",
            String::from_utf8_lossy(&native.stderr)
        );
        assert_eq!(
            candidate_is_valid(&root, name, source),
            native.status.success(),
            "{name}: Ruby={} worker classification differs",
            String::from_utf8_lossy(&native.stderr)
        );
        compared += 1;
    }
    fs::remove_dir_all(root).unwrap();
    assert_eq!(compared, 13);
}
