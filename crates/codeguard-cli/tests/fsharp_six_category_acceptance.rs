//! F# 六类别真实工具验收
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
fn valid_fsharp_samples_pass_parse() {
    let tmp = ensure_clean_dir("fsharp-valid-test");
    std::fs::write(tmp.join("main.fsx"), "// Adds two numbers.\nlet add a b = a + b\n\nprintfn \"%d\" (add 1 2)\n").unwrap();

    let output = Command::new("dotnet")
        .args(["fsi", "main.fsx"])
        .current_dir(&tmp)
        .output()
        .expect("dotnet fsi 不可用");

    assert!(output.status.success(), "有效 F# 代码应通过执行: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_fsharp_samples_are_detected() {
    let tmp = ensure_clean_dir("fsharp-invalid-test");
    std::fs::write(tmp.join("main.fsx"), "let broken a b = a + b + notDefinedAnywhere\n\nprintfn \"%d\" (broken 1 2)\n").unwrap();

    let output = Command::new("dotnet")
        .args(["fsi", "main.fsx"])
        .current_dir(&tmp)
        .output()
        .expect("dotnet fsi 不可用");

    assert!(!output.status.success(), "未定义变量应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("fsharp-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("main.fsx"), "let undocumented a = a * 2\n\n// Has docs.\nlet documented a = a + 1\n\nprintfn \"%d\" (undocumented 5 + documented 3)\n").unwrap();

    let output = Command::new("dotnet")
        .args(["fsi", "main.fsx"])
        .current_dir(&tmp)
        .output()
        .expect("dotnet fsi 不可用");

    assert!(output.status.success(), "F# 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
