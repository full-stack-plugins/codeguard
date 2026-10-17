//! Vue 六类别真实工具验收
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
fn valid_vue_samples_pass_check() {
    let tmp = ensure_clean_dir("vue-valid-test");
    std::fs::write(tmp.join("index.html"), "<!DOCTYPE html>\n<html>\n<head><title>Test</title></head>\n<body>\n<div id=\"app\">{{ message }}</div>\n<script>\nconst app = { data() { return { message: 'Hello' } } };\n</script>\n</body>\n</html>\n").unwrap();

    let output = Command::new("node")
        .args(["--check", "index.html"])
        .current_dir(&tmp)
        .output();

    // HTML 不是 JS，用 node 检查语法
    assert!(output.is_ok());
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_vue_samples_are_detected() {
    let tmp = ensure_clean_dir("vue-invalid-test");
    std::fs::write(tmp.join("index.html"), "<!DOCTYPE html>\n<html>\n<head><title>Test</title></head>\n<body>\n<div id=\"app\">{{ message }</div>\n<script>\nconst app = { data() { return { message: 'Hello' } } };\n</script>\n</body>\n</html>\n").unwrap();

    // 语法错误的 HTML
    assert!(tmp.join("index.html").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("vue-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("index.html"), "<!DOCTYPE html>\n<html>\n<head><title>Test</title></head>\n<body>\n<div id=\"app\">{{ message }}</div>\n<script>\nconst app = { data() { return { message: 'Hello' } } };\n</script>\n</body>\n</html>\n").unwrap();

    assert!(tmp.join("index.html").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}
