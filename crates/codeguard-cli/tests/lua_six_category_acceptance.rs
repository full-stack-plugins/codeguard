//! Lua 六类别真实工具验收
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
fn valid_lua_samples_pass_syntax() {
    let tmp = ensure_clean_dir("lua-valid-test");
    std::fs::write(tmp.join("main.lua"), "-- Adds two numbers.\nlocal function add(a, b)\n    return a + b\nend\n\nprint(add(1, 2))\n").unwrap();

    let output = Command::new("luac")
        .args(["-p", "main.lua"])
        .current_dir(&tmp)
        .output()
        .expect("luac 不可用");

    assert!(output.status.success(), "有效 Lua 代码应通过语法检查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_lua_samples_are_detected() {
    let tmp = ensure_clean_dir("lua-invalid-test");
    std::fs::write(tmp.join("main.lua"), "local function broken(\n    return 42\nend\n").unwrap();

    let output = Command::new("luac")
        .args(["-p", "main.lua"])
        .current_dir(&tmp)
        .output()
        .expect("luac 不可用");

    assert!(!output.status.success(), "语法错误应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("lua-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("main.lua"), "local function undocumented(a)\n    return a * 2\nend\n\n-- Has docs.\nlocal function documented(a)\n    return a + 1\nend\n\nprint(undocumented(5) + documented(3))\n").unwrap();

    let output = Command::new("luac")
        .args(["-p", "main.lua"])
        .current_dir(&tmp)
        .output()
        .expect("luac 不可用");

    assert!(output.status.success(), "Lua 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
