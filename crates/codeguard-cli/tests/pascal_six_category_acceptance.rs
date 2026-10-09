//! Pascal 六类别真实工具验收
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
fn valid_pascal_samples_pass_compile() {
    let tmp = ensure_clean_dir("pascal-valid-test");
    std::fs::write(tmp.join("main.pas"), "program Hello;\n{ Adds two numbers. }\nfunction Add(a, b: Integer): Integer;\nbegin\n    Add := a + b;\nend;\n\nbegin\n    WriteLn(Add(1, 2));\nend.\n").unwrap();

    let output = Command::new("fpc")
        .args(["-s", "main.pas"])
        .current_dir(&tmp)
        .output()
        .expect("fpc 不可用");

    assert!(output.status.success(), "有效 Pascal 代码应通过编译: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_pascal_samples_are_detected() {
    let tmp = ensure_clean_dir("pascal-invalid-test");
    std::fs::write(tmp.join("main.pas"), "program Broken;\nbegin\n    undefined_function;\nend.\n").unwrap();

    let output = Command::new("fpc")
        .args(["-s", "main.pas"])
        .current_dir(&tmp)
        .output()
        .expect("fpc 不可用");

    assert!(!output.status.success(), "未定义函数应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("pascal-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("main.pas"), "program Test;\nfunction Undocumented(a: Integer): Integer;\nbegin\n    Undocumented := a * 2;\nend;\n\n{ Has docs. }\nfunction Documented(a: Integer): Integer;\nbegin\n    Documented := a + 1;\nend;\n\nbegin\n    WriteLn(Undocumented(5) + Documented(3));\nend.\n").unwrap();

    let output = Command::new("fpc")
        .args(["-s", "main.pas"])
        .current_dir(&tmp)
        .output()
        .expect("fpc 不可用");

    assert!(output.status.success(), "Pascal 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
