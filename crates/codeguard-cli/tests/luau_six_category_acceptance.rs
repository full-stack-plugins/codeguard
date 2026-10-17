//! Luau 六类别真实工具验收
//! 验收标准：正例通过 + 负例检出 + 格式≠注释合规

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

fn luau_bin() -> String {
    std::env::var("CODEGUARD_LUAU_ANALYZE_BIN").unwrap_or_else(|_| "luau-analyze".to_string())
}

/// 正例通过
#[test]
fn valid_luau_samples_pass_analyze() {
    let tmp = ensure_clean_dir("luau-valid-test");
    std::fs::write(tmp.join("main.luau"), "-- Adds two numbers.\nlocal function add(a, b)\n    return a + b\nend\n\nprint(add(1, 2))\n").unwrap();

    let output = Command::new(luau_bin())
        .args(["main.luau"])
        .current_dir(&tmp)
        .output()
        .expect("luau 不可用");

    assert!(output.status.success(), "有效 Luau 代码应通过分析: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_luau_samples_are_detected() {
    let tmp = ensure_clean_dir("luau-invalid-test");
    std::fs::write(tmp.join("main.luau"), "local function broken(a, b)\n    return a + b + notDefinedAnywhere\nend\n\nprint(broken(1, 2))\n").unwrap();

    let output = Command::new(luau_bin())
        .args(["main.luau"])
        .current_dir(&tmp)
        .output()
        .expect("luau 不可用");

    assert!(!output.status.success(), "未定义变量应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("luau-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("main.luau"), "local function undocumented(a)\n    return a * 2\nend\n\n-- Has docs.\nlocal function documented(a)\n    return a + 1\nend\n\nprint(undocumented(5) + documented(3))\n").unwrap();

    let output = Command::new(luau_bin())
        .args(["main.luau"])
        .current_dir(&tmp)
        .output()
        .expect("luau 不可用");

    assert!(output.status.success(), "Luau 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
