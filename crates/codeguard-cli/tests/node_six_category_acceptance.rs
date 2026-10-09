use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_node_samples_pass_syntax_check() {
    let tmp = ensure_clean_dir("node-valid-test");
    std::fs::write(tmp.join("main.js"), "/** Adds two numbers. */\nfunction add(a, b) {\n  return a + b;\n}\n\nmodule.exports = { add };\n").unwrap();

    let output = Command::new("node")
        .args(["--check", "main.js"])
        .current_dir(&tmp)
        .output()
        .expect("node 不可用");

    assert!(
        output.status.success(),
        "有效 JS 代码应通过 node --check: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_node_samples_are_detected() {
    let tmp = ensure_clean_dir("node-invalid-test");
    std::fs::write(tmp.join("main.js"), "function broken( {\n  return 42\n}\n").unwrap();

    let output = Command::new("node")
        .args(["--check", "main.js"])
        .current_dir(&tmp)
        .output()
        .expect("node 不可用");

    assert!(!output.status.success(), "语法错误应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("node-comment-test");
    // 格式正确但缺少文档——node --check 不检查文档
    std::fs::write(tmp.join("main.js"), "function undocumented() {\n  return 42;\n}\n\n/** This has docs. */\nfunction documented() {\n  return 0;\n}\n\nmodule.exports = { undocumented, documented };\n").unwrap();

    let output = Command::new("node")
        .args(["--check", "main.js"])
        .current_dir(&tmp)
        .output()
        .expect("node 不可用");

    assert!(
        output.status.success(),
        "node --check 应通过（语法正确），但文档合规需另查: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}
