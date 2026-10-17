//! Dart 六类别真实工具验收
//! 验收标准：正例通过 + 负例检出 + 格式≠注释合规

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

fn dart_bin() -> String {
    std::env::var("CODEGUARD_DART_BIN").unwrap_or_else(|_| "dart".to_string())
}

/// 正例通过
#[test]
fn valid_dart_samples_pass_analyze() {
    let tmp = ensure_clean_dir("dart-valid-test");
    std::fs::write(tmp.join("main.dart"), "// Adds two numbers.\nint add(int a, int b) {\n  return a + b;\n}\n\nvoid main() {\n  print(add(1, 2));\n}\n").unwrap();

    let output = Command::new(dart_bin())
        .args(["analyze", "main.dart"])
        .current_dir(&tmp)
        .output()
        .expect("dart 不可用");

    assert!(output.status.success(), "有效 Dart 代码应通过分析: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_dart_samples_are_detected() {
    let tmp = ensure_clean_dir("dart-invalid-test");
    std::fs::write(tmp.join("main.dart"), "int broken(int a, int b) {\n  return a + b + notDefinedAnywhere;\n}\n\nvoid main() {\n  print(broken(1, 2));\n}\n").unwrap();

    let output = Command::new(dart_bin())
        .args(["analyze", "main.dart"])
        .current_dir(&tmp)
        .output()
        .expect("dart 不可用");

    assert!(!output.status.success(), "未定义变量应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("dart-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("main.dart"), "int undocumented(int a) {\n  return a * 2;\n}\n\n// Has docs.\nint documented(int a) {\n  return a + 1;\n}\n\nvoid main() {\n  print(undocumented(5) + documented(3));\n}\n").unwrap();

    let output = Command::new(dart_bin())
        .args(["analyze", "main.dart"])
        .current_dir(&tmp)
        .output()
        .expect("dart 不可用");

    assert!(output.status.success(), "Dart 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
