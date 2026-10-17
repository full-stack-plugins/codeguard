//! Crystal 六类别真实工具验收
//! 验收标准：正例通过 + 负例检出 + 格式≠注释合规

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

fn crystal_bin() -> String {
    std::env::var("CODEGUARD_CRYSTAL_BIN").unwrap_or_else(|_| "crystal".to_string())
}

/// 正例通过
#[test]
fn valid_crystal_samples_pass_compile() {
    let tmp = ensure_clean_dir("crystal-valid-test");
    std::fs::write(tmp.join("main.cr"), "# Adds two numbers.\ndef add(a, b)\n  a + b\nend\n\nputs add(1, 2)\n").unwrap();

    let output = Command::new(crystal_bin())
        .args(["build", "--no-codegen", "main.cr"])
        .current_dir(&tmp)
        .output()
        .expect("crystal 不可用");

    assert!(output.status.success(), "有效 Crystal 代码应通过编译: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_crystal_samples_are_detected() {
    let tmp = ensure_clean_dir("crystal-invalid-test");
    std::fs::write(tmp.join("main.cr"), "def broken(a, b)\n  a + b + not_defined_anywhere\nend\n\nputs broken(1, 2)\n").unwrap();

    let output = Command::new(crystal_bin())
        .args(["build", "--no-codegen", "main.cr"])
        .current_dir(&tmp)
        .output()
        .expect("crystal 不可用");

    assert!(!output.status.success(), "未定义变量应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("crystal-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("main.cr"), "def undocumented(a)\n  a * 2\nend\n\n# Has docs.\ndef documented(a)\n  a + 1\nend\n\nputs undocumented(5) + documented(3)\n").unwrap();

    let output = Command::new(crystal_bin())
        .args(["build", "--no-codegen", "main.cr"])
        .current_dir(&tmp)
        .output()
        .expect("crystal 不可用");

    assert!(output.status.success(), "Crystal 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
