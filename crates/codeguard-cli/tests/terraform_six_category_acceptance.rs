//! Terraform 六类别真实工具验收
//! 验收标准：正例通过 + 负例检出 + 格式≠注释合规

use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

fn terraform_bin() -> String {
    std::env::var("CODEGUARD_TERRAFORM_BIN").unwrap_or_else(|_| "terraform".to_string())
}

/// 正例通过
#[test]
fn valid_terraform_samples_pass_validate() {
    let tmp = ensure_clean_dir("terraform-valid-test");
    std::fs::write(tmp.join("main.tf"), "# Adds a resource.\nresource \"null_resource\" \"example\" {\n  triggers = {\n    value = \"test\"\n  }\n}\n").unwrap();

    let output = Command::new(terraform_bin())
        .args(["init", "-backend=false"])
        .current_dir(&tmp)
        .output()
        .expect("terraform 不可用");

    let output2 = Command::new(terraform_bin())
        .args(["validate"])
        .current_dir(&tmp)
        .output()
        .expect("terraform 不可用");

    assert!(output2.status.success(), "有效 Terraform 代码应通过验证: {:?}", output2);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_terraform_samples_are_detected() {
    let tmp = ensure_clean_dir("terraform-invalid-test");
    std::fs::write(tmp.join("main.tf"), "resource \"null_resource\" \"example\" {\n  triggers = {\n    value = \"test\"\n  }\n").unwrap();

    let output = Command::new(terraform_bin())
        .args(["init", "-backend=false"])
        .current_dir(&tmp)
        .output()
        .expect("terraform 不可用");

    let output2 = Command::new(terraform_bin())
        .args(["validate"])
        .current_dir(&tmp)
        .output()
        .expect("terraform 不可用");

    assert!(!output2.status.success(), "语法错误应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("terraform-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("main.tf"), "resource \"null_resource\" \"undocumented\" {\n  triggers = {\n    value = \"test\"\n  }\n}\n\n# Has docs.\nresource \"null_resource\" \"documented\" {\n  triggers = {\n    value = \"test\"\n  }\n}\n").unwrap();

    let output = Command::new(terraform_bin())
        .args(["init", "-backend=false"])
        .current_dir(&tmp)
        .output()
        .expect("terraform 不可用");

    let output2 = Command::new(terraform_bin())
        .args(["validate"])
        .current_dir(&tmp)
        .output()
        .expect("terraform 不可用");

    assert!(output2.status.success(), "Terraform 应通过（语法正确），但注释合规需另查: {:?}", output2);
    std::fs::remove_dir_all(&tmp).unwrap();
}
