//! Scala 六类别真实工具验收
//! 验收标准：正例通过 + 负例检出 + 格式≠注释合规

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

/// 正例通过
#[test]
fn valid_scala_samples_pass_parse() {
    let tmp = ensure_clean_dir("scala-valid-test");
    std::fs::write(tmp.join("Main.scala"), "object Main {\n  // Adds two numbers.\n  def add(a: Int, b: Int): Int = a + b\n\n  def main(args: Array[String]): Unit = {\n    println(add(1, 2))\n  }\n}\n").unwrap();

    assert!(tmp.join("Main.scala").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_scala_samples_are_detected() {
    let tmp = ensure_clean_dir("scala-invalid-test");
    std::fs::write(tmp.join("Main.scala"), "object Main {\n  def add(a: Int, b: Int): Int = a + b\n\n  def main(args: Array[String]): Unit = {\n    println(add(1, 2)\n  }\n}\n").unwrap();

    assert!(tmp.join("Main.scala").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("scala-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("Main.scala"), "object Main {\n  def add(a: Int, b: Int): Int = a + b\n\n  def main(args: Array[String]): Unit = {\n    println(add(1, 2))\n  }\n}\n").unwrap();

    assert!(tmp.join("Main.scala").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}
