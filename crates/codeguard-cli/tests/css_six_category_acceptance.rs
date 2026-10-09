use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_css_samples_pass_lint() {
    let tmp = ensure_clean_dir("css-valid-test");
    std::fs::write(tmp.join("style.css"), "/* Main styles */\nbody {\n    margin: 0;\n    padding: 0;\n    font-family: sans-serif;\n}\n\n.container {\n    max-width: 1200px;\n    margin: 0 auto;\n}\n").unwrap();

    let _ = Command::new("npx")
        .args(["prettier", "--write", "style.css"])
        .current_dir(&tmp)
        .output();

    let output = Command::new("npx")
        .args(["prettier", "--check", "style.css"])
        .current_dir(&tmp)
        .output()
        .expect("prettier 不可用");

    assert!(
        output.status.success(),
        "有效 CSS 应通过 prettier: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_css_samples_are_detected() {
    let tmp = ensure_clean_dir("css-invalid-test");
    std::fs::write(
        tmp.join("style.css"),
        "body {\n    margin: 0\n    padding: 0;\n    font-family: sans-serif;\n}\n",
    )
    .unwrap();

    let output = Command::new("npx")
        .args(["prettier", "--check", "style.css"])
        .current_dir(&tmp)
        .output()
        .expect("prettier 不可用");

    assert!(!output.status.success(), "格式错误应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("css-comment-test");
    // 格式正确但缺少注释——prettier 不检查注释
    std::fs::write(tmp.join("style.css"), ".header {\n    background: #333;\n    color: white;\n}\n\n.footer {\n    background: #666;\n    color: white;\n}\n").unwrap();

    let _ = Command::new("npx")
        .args(["prettier", "--write", "style.css"])
        .current_dir(&tmp)
        .output();

    let output = Command::new("npx")
        .args(["prettier", "--check", "style.css"])
        .current_dir(&tmp)
        .output()
        .expect("prettier 不可用");

    assert!(
        output.status.success(),
        "CSS 应通过（格式正确），但注释合规需另查: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}
