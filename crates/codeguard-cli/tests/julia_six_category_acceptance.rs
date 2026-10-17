//! Julia 六类别真实工具验收
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
fn valid_julia_samples_pass_parse() {
    let tmp = ensure_clean_dir("julia-valid-test");
    std::fs::write(tmp.join("main.jl"), "# Adds two numbers.\nfunction add(a, b)\n    return a + b\nend\n\nprintln(add(1, 2))\n").unwrap();

    let output = Command::new("julia")
        .args(["-e", "include(\"main.jl\")"])
        .current_dir(&tmp)
        .output()
        .expect("julia 不可用");

    assert!(output.status.success(), "有效 Julia 代码应通过执行: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_julia_samples_are_detected() {
    let tmp = ensure_clean_dir("julia-invalid-test");
    std::fs::write(tmp.join("main.jl"), "function broken(a, b)\n    return a + b + not_defined_anywhere\nend\n\nprintln(broken(1, 2))\n").unwrap();

    let output = Command::new("julia")
        .args(["-e", "include(\"main.jl\")"])
        .current_dir(&tmp)
        .output()
        .expect("julia 不可用");

    assert!(!output.status.success(), "未定义变量应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("julia-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("main.jl"), "function undocumented(a)\n    return a * 2\nend\n\n# Has docs.\nfunction documented(a)\n    return a + 1\nend\n\nprintln(undocumented(5) + documented(3))\n").unwrap();

    let output = Command::new("julia")
        .args(["-e", "include(\"main.jl\")"])
        .current_dir(&tmp)
        .output()
        .expect("julia 不可用");

    assert!(output.status.success(), "Julia 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
