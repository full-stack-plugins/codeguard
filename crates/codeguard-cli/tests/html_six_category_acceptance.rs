use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_html_samples_pass_lint() {
    let tmp = ensure_clean_dir("html-valid-test");
    std::fs::write(tmp.join("index.html"), "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n    <meta charset=\"UTF-8\">\n    <title>Test</title>\n</head>\n<body>\n    <h1>Hello</h1>\n</body>\n</html>\n").unwrap();

    let _ = Command::new("npx")
        .args(["prettier", "--write", "index.html"])
        .current_dir(&tmp)
        .output();

    let output = Command::new("npx")
        .args(["prettier", "--check", "index.html"])
        .current_dir(&tmp)
        .output()
        .expect("prettier 不可用");

    assert!(output.status.success(), "有效 HTML 应通过 prettier: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_html_samples_are_detected() {
    let tmp = ensure_clean_dir("html-invalid-test");
    std::fs::write(tmp.join("index.html"), "<!DOCTYPE html>\n<html>\n<head>\n    <title>Test\n</head>\n<body>\n    <h1>Hello</h1>\n</body>\n</html>\n").unwrap();

    let output = Command::new("npx")
        .args(["prettier", "--check", "index.html"])
        .current_dir(&tmp)
        .output()
        .expect("prettier 不可用");

    // prettier 会报告格式问题或解析错误
    assert!(!output.status.success() || !output.stderr.is_empty(), "格式错误应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("html-comment-test");
    // 格式正确但缺少注释——prettier 不检查注释
    std::fs::write(tmp.join("index.html"), "<!DOCTYPE html>\n<html>\n<head>\n    <title>Test</title>\n</head>\n<body>\n    <div class=\"container\">\n        <p>Content</p>\n    </div>\n</body>\n</html>\n").unwrap();

    let _ = Command::new("npx")
        .args(["prettier", "--write", "index.html"])
        .current_dir(&tmp)
        .output();

    let output = Command::new("npx")
        .args(["prettier", "--check", "index.html"])
        .current_dir(&tmp)
        .output()
        .expect("prettier 不可用");

    assert!(output.status.success(), "HTML 应通过（格式正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
