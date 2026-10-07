//! PowerShell 六类别真实工具验收
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
fn valid_powershell_samples_pass_parse() {
    let tmp = ensure_clean_dir("powershell-valid-test");
    std::fs::write(tmp.join("main.ps1"), "# Adds two numbers.\nfunction Add($a, $b) {\n    return $a + $b\n}\n\nWrite-Host (Add 1 2)\n").unwrap();

    let output = Command::new("pwsh")
        .args(["-NoProfile", "-File", "main.ps1"])
        .current_dir(&tmp)
        .output()
        .expect("pwsh 不可用");

    assert!(output.status.success(), "有效 PowerShell 代码应通过执行: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_powershell_samples_are_detected() {
    let tmp = ensure_clean_dir("powershell-invalid-test");
    std::fs::write(tmp.join("main.ps1"), "function Broken($a, $b) {\n    return $a + $b\n}\n\nWrite-Host (Broken 1 2\n").unwrap();

    let output = Command::new("pwsh")
        .args(["-NoProfile", "-File", "main.ps1"])
        .current_dir(&tmp)
        .output()
        .expect("pwsh 不可用");

    assert!(!output.status.success(), "未定义变量应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("powershell-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("main.ps1"), "function Undocumented($a) {\n    return $a * 2\n}\n\n# Has docs.\nfunction Documented($a) {\n    return $a + 1\n}\n\nWrite-Host ((Undocumented 5) + (Documented 3))\n").unwrap();

    let output = Command::new("pwsh")
        .args(["-NoProfile", "-File", "main.ps1"])
        .current_dir(&tmp)
        .output()
        .expect("pwsh 不可用");

    assert!(output.status.success(), "PowerShell 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
