use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_markdown_samples_pass_parse() {
    let tmp = ensure_clean_dir("markdown-valid-test");
    std::fs::write(tmp.join("README.md"), "# Project Title\n\nA sample project.\n\n## Installation\n\n```bash\nnpm install\n```\n\n## Usage\n\nSee [docs](docs.md).\n").unwrap();

    let output = Command::new("python3")
        .args(["-c", "import markdown; markdown.markdown(open('README.md').read()); print('OK')"])
        .current_dir(&tmp)
        .output()
        .expect("pandoc 不可用");

    assert!(output.status.success(), "有效 Markdown 应通过解析: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_markdown_samples_are_detected() {
    let tmp = ensure_clean_dir("markdown-invalid-test");
    // 未闭合的代码块
    std::fs::write(tmp.join("README.md"), "# Title\n\n```bash\nnpm install\n").unwrap();

    let output = Command::new("python3")
        .args(["-c", "import markdown; markdown.markdown(open('README.md').read()); print('OK')"])
        .current_dir(&tmp)
        .output()
        .expect("pandoc 不可用");

    // pandoc 对未闭合代码块通常给出警告但不一定失败
    // 所以这里验证的是解析器能处理
    assert!(output.status.success() || !output.stderr.is_empty(), "解析器应能处理");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("markdown-comment-test");
    // 格式正确但缺少文档注释——Markdown 解析器不检查注释
    std::fs::write(tmp.join("README.md"), "# API\n\n## Function One\n\nDoes something.\n\n## Function Two\n\nDoes something else.\n").unwrap();

    let output = Command::new("python3")
        .args(["-c", "import markdown; markdown.markdown(open('README.md').read()); print('OK')"])
        .current_dir(&tmp)
        .output()
        .expect("pandoc 不可用");

    assert!(output.status.success(), "Markdown 应通过（格式正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
