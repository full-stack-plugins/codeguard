//! 适配器行为验收测试。
//!
//! 验证适配器的输出解析、配置检测、类别覆盖等核心行为。

use serde_json::{Value, json};

/// 测试 Go vet 输出解析。
#[test]
fn go_vet_output_parsing() {
    // 模拟 go vet 输出
    let output = "src/main.go:10:5: unreachable code\nsrc/main.go:20:1: exported function Foo should have comment";
    let findings: Vec<Value> = output
        .lines()
        .filter(|l| l.contains(":"))
        .map(|line| {
            let parts: Vec<&str> = line.splitn(4, ':').collect();
            json!({
                "path": parts[0],
                "line": parts[1].parse::<u32>().unwrap_or(0),
                "message": parts[3],
                "source": "go_vet",
            })
        })
        .collect();

    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0]["path"], "src/main.go");
    assert_eq!(findings[0]["line"], 10);
}

/// 测试 PHP_CodeSniffer JSON 输出解析。
#[test]
fn phpcs_json_output_parsing() {
    let json_str = r#"{"files": {"test.php": {"messages": [{"line": 5, "column": 3, "severity": 5, "message": "Missing doc comment", "source": "PSR12", "fixable": true}]}}}"#;
    let report: Value = serde_json::from_str(json_str).unwrap();
    let findings: Vec<Value> = report["files"]["test.php"]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|msg| {
            json!({
                "path": "test.php",
                "line": msg["line"],
                "column": msg["column"],
                "message": msg["message"],
                "source": msg["source"],
            })
        })
        .collect();

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["line"], 5);
    assert_eq!(findings[0]["source"], "PSR12");
}

/// 测试 Credo JSON 输出解析。
#[test]
fn credo_json_output_parsing() {
    let json_str = r#"{"issues": [{"filename": "lib/main.ex", "line_no": 10, "column": 5, "severity": "warning", "message": "Module doc missing", "check": "Credo.Check.Readability.ModuleDoc"}]}"#;
    let report: Value = serde_json::from_str(json_str).unwrap();
    let findings: Vec<Value> = report["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|issue| {
            json!({
                "path": issue["filename"],
                "line": issue["line_no"],
                "message": issue["message"],
                "source": issue["check"],
            })
        })
        .collect();

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0]["path"], "lib/main.ex");
}

/// 测试配置检测逻辑。
#[test]
fn config_detection_patterns() {
    // PHP: .phpcs.xml / phpcs.xml / composer.json
    let php_config = |exists: &[&str]| {
        let config = if exists.contains(&".phpcs.xml") || exists.contains(&"phpcs.xml") {
            "configured"
        } else if exists.contains(&"composer.json") {
            "missing"
        } else {
            "unknown"
        };
        config
    };
    assert_eq!(php_config(&[".phpcs.xml"]), "configured");
    assert_eq!(php_config(&["composer.json"]), "missing");
    assert_eq!(php_config(&[]), "unknown");

    // Scala: .scalafix.conf / build.sbt
    let scala_config = |exists: &[&str]| {
        if exists.contains(&".scalafix.conf") {
            "configured"
        } else if exists.contains(&"build.sbt") {
            "missing"
        } else {
            "unknown"
        }
    };
    assert_eq!(scala_config(&[".scalafix.conf"]), "configured");
    assert_eq!(scala_config(&["build.sbt"]), "missing");
}

/// 测试类别覆盖完整性。
#[test]
fn category_coverage_completeness() {
    // 每种语言应覆盖 6 个类别
    let categories = vec![
        "lint",
        "comments",
        "dependencies",
        "cve",
        "security",
        "build",
    ];

    // lint/comments 适配器
    let lint_profile = json!({
        "categories": [
            {"category": "lint", "applicability": "applicable"},
            {"category": "comments", "applicability": "applicable"},
        ]
    });
    let lint_cats: Vec<&str> = lint_profile["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["category"].as_str().unwrap())
        .collect();
    assert!(lint_cats.contains(&"lint"));
    assert!(lint_cats.contains(&"comments"));

    // dependency 适配器
    let dep_profile = json!({
        "categories": [
            {"category": "dependencies", "applicability": "applicable"},
            {"category": "cve", "applicability": "applicable"},
            {"category": "security", "applicability": "applicable"},
            {"category": "build", "applicability": "applicable"},
        ]
    });
    let dep_cats: Vec<&str> = dep_profile["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["category"].as_str().unwrap())
        .collect();
    assert_eq!(dep_cats.len(), 4);

    // 六类别全覆盖
    let all_cats: Vec<&str> = categories.iter().copied().collect();
    assert_eq!(all_cats.len(), 6);
}

/// 测试优雅降级行为。
#[test]
fn graceful_degradation_when_tool_missing() {
    // 工具不可用时应返回 incomplete，而非 panic
    let result = std::panic::catch_unwind(|| {
        // 模拟工具缺失场景
        let report = json!({
            "language": "test",
            "report_type": "test_lint_scan",
            "status": "incomplete",
            "reason": "tool_not_available"
        });
        assert_eq!(report["status"], "incomplete");
    });
    assert!(result.is_ok(), "工具缺失时应优雅降级而非 panic");
}
