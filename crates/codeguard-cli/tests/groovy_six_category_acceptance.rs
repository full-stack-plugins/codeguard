//! Groovy 六类别真实工具验收
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
fn valid_groovy_samples_pass_compile() {
    let tmp = ensure_clean_dir("groovy-valid-test");
    std::fs::write(tmp.join("main.groovy"), "// Adds two numbers.\ndef add(a, b) {\n    return a + b\n}\n\nprintln add(1, 2)\n").unwrap();

    let output = Command::new("groovy")
        .args(["main.groovy"])
        .current_dir(&tmp)
        .output()
        .expect("groovy 不可用");

    assert!(output.status.success(), "有效 Groovy 代码应通过执行: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_groovy_samples_are_detected() {
    let tmp = ensure_clean_dir("groovy-invalid-test");
    std::fs::write(tmp.join("main.groovy"), "def broken( {\n    return 42\n}\n").unwrap();

    let output = Command::new("groovy")
        .args(["main.groovy"])
        .current_dir(&tmp)
        .output()
        .expect("groovy 不可用");

    assert!(!output.status.success(), "语法错误应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("groovy-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("main.groovy"), "def undocumented(a) {\n    return a * 2\n}\n\n// Has docs.\ndef documented(a) {\n    return a + 1\n}\n\nprintln undocumented(5) + documented(3)\n").unwrap();

    let output = Command::new("groovy")
        .args(["main.groovy"])
        .current_dir(&tmp)
        .output()
        .expect("groovy 不可用");

    assert!(output.status.success(), "Groovy 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
