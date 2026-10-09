//! Elm 六类别真实工具验收
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
fn valid_elm_samples_pass_parse() {
    let tmp = ensure_clean_dir("elm-valid-test");
    // 使用 elm make 直接编译单文件
    std::fs::write(tmp.join("Main.elm"), "module Main exposing (main)\n\nimport Html exposing (text)\n\n-- Adds two numbers.\nadd : Int -> Int -> Int\nadd a b =\n    a + b\n\nmain =\n    text (String.fromInt (add 1 2))\n").unwrap();

    // 初始化 elm 项目
    let _ = Command::new("sh")
        .args(["-c", "echo 'Y' | elm init"])
        .current_dir(&tmp)
        .output();

    let output = Command::new("elm")
        .args(["make", "Main.elm", "--output=/dev/null"])
        .current_dir(&tmp)
        .output()
        .expect("elm 不可用");

    assert!(output.status.success(), "有效 Elm 代码应通过编译: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_elm_samples_are_detected() {
    let tmp = ensure_clean_dir("elm-invalid-test");
    std::fs::write(tmp.join("Main.elm"), "module Main exposing (main)\n\nimport Html exposing (text)\n\nbroken : Int -> Int -> Int\nbroken a b =\n    a + b + notDefinedAnywhere\n\nmain =\n    text (String.fromInt (broken 1 2))\n").unwrap();

    let _ = Command::new("sh")
        .args(["-c", "echo 'Y' | elm init"])
        .current_dir(&tmp)
        .output();

    let output = Command::new("elm")
        .args(["make", "Main.elm", "--output=/dev/null"])
        .current_dir(&tmp)
        .output()
        .expect("elm 不可用");

    assert!(!output.status.success(), "未定义变量应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("elm-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("Main.elm"), "module Main exposing (main)\n\nimport Html exposing (text)\n\nundocumented : Int -> Int\nundocumented a =\n    a * 2\n\n-- Has docs.\ndocumented : Int -> Int\ndocumented a =\n    a + 1\n\nmain =\n    text (String.fromInt (undocumented 5 + documented 3))\n").unwrap();

    let _ = Command::new("sh")
        .args(["-c", "echo 'Y' | elm init"])
        .current_dir(&tmp)
        .output();

    let output = Command::new("elm")
        .args(["make", "Main.elm", "--output=/dev/null"])
        .current_dir(&tmp)
        .output()
        .expect("elm 不可用");

    assert!(output.status.success(), "Elm 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
