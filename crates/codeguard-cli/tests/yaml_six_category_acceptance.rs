use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_yaml_samples_pass_lint() {
    let tmp = ensure_clean_dir("yaml-valid-test");
    std::fs::write(tmp.join("config.yaml"), "name: my-app\nversion: 1.0.0\ndescription: A sample application\n\nsettings:\n  debug: false\n  timeout: 30\n  retries: 3\n\n# Database configuration\ndatabase:\n  host: localhost\n  port: 5432\n  name: mydb\n").unwrap();

    let output = Command::new("yamllint")
        .args(["config.yaml"])
        .current_dir(&tmp)
        .output()
        .expect("yamllint 不可用");

    assert!(
        output.status.success(),
        "有效 YAML 应通过 lint: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_yaml_samples_are_detected() {
    let tmp = ensure_clean_dir("yaml-invalid-test");
    std::fs::write(tmp.join("config.yaml"), "name: my-app\nversion: 1.0.0\nsettings:\n  debug: false\n   timeout: 30  # inconsistent indentation\n  retries: 3\n").unwrap();

    let output = Command::new("yamllint")
        .args(["config.yaml"])
        .current_dir(&tmp)
        .output()
        .expect("yamllint 不可用");

    assert!(!output.status.success(), "缩进错误应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("yaml-comment-test");
    // 格式正确但缺少注释——yamllint 默认不强制注释
    std::fs::write(tmp.join("config.yaml"), "server:\n  host: 0.0.0.0\n  port: 8080\n\nlogging:\n  level: info\n  format: json\n\nfeatures:\n  - auth\n  - cache\n  - metrics\n").unwrap();

    let output = Command::new("yamllint")
        .args(["config.yaml"])
        .current_dir(&tmp)
        .output()
        .expect("yamllint 不可用");

    assert!(
        output.status.success(),
        "YAML 应通过（格式正确），但注释合规需另查: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}
