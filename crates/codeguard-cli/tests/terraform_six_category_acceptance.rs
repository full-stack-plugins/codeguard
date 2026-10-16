//! Terraform 真实工具语法验收：使用内置 terraform_data，不下载外部 provider。
//! 正例、错误配置、修复复验和缺少注释样本均实际调用原生 Terraform。

use std::{
    path::Path,
    process::{Command, Output},
};

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

fn terraform_bin() -> String {
    std::env::var("CODEGUARD_TERRAFORM_BIN").unwrap_or_else(|_| "terraform".to_string())
}

fn run(tmp: &Path, args: &[&str]) -> Output {
    Command::new(terraform_bin())
        .args(args)
        .env("TF_IN_AUTOMATION", "1")
        .env("CHECKPOINT_DISABLE", "1")
        .current_dir(tmp)
        .output()
        .expect("terraform 不可用")
}

fn initialize(tmp: &Path) {
    let output = run(
        tmp,
        &["init", "-backend=false", "-input=false", "-no-color"],
    );
    assert!(
        output.status.success(),
        "Terraform 初始化必须成功，不能把工具缺失当作负例检出: {output:?}"
    );
}

const VALID: &str = "# Stores an example value.\nresource \"terraform_data\" \"example\" {\n  input = \"test\"\n}\n";

#[test]
fn valid_terraform_samples_pass_validate() {
    let tmp = ensure_clean_dir("terraform-valid-test");
    std::fs::write(tmp.join("main.tf"), VALID).unwrap();
    initialize(&tmp);
    let output = run(&tmp, &["validate", "-no-color"]);
    assert!(
        output.status.success(),
        "有效 Terraform 代码应通过验证: {output:?}"
    );
    std::fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn invalid_terraform_samples_are_detected() {
    let tmp = ensure_clean_dir("terraform-invalid-test");
    let file = tmp.join("main.tf");
    std::fs::write(&file, VALID).unwrap();
    initialize(&tmp);
    // 保持有效 HCL，由内置 provider 的真实 schema 拒绝未知属性。
    std::fs::write(
        &file,
        "resource \"terraform_data\" \"example\" {\n  unsupported = true\n}\n",
    )
    .unwrap();
    let output = run(&tmp, &["validate", "-no-color"]);
    assert!(!output.status.success(), "错误配置应被检出");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unsupported argument"),
        "失败必须来自目标配置错误: {output:?}"
    );
    // 同一个初始化目录中修复并复验，排除缺工具/缺 provider 造成的假阴性。
    std::fs::write(&file, VALID).unwrap();
    let repaired = run(&tmp, &["validate", "-no-color"]);
    assert!(repaired.status.success(), "修复后应重新通过: {repaired:?}");
    std::fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("terraform-comment-test");
    std::fs::write(tmp.join("main.tf"), "resource \"terraform_data\" \"undocumented\" {\n  input = \"test\"\n}\n\n# Has docs.\nresource \"terraform_data\" \"documented\" {\n  input = \"test\"\n}\n").unwrap();
    initialize(&tmp);
    let output = run(&tmp, &["validate", "-no-color"]);
    assert!(
        output.status.success(),
        "Terraform 应通过语法验证，但注释合规需另查: {output:?}"
    );
    std::fs::remove_dir_all(tmp).unwrap();
}
