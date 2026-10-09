use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_toml_samples_pass_parse() {
    let tmp = ensure_clean_dir("toml-valid-test");
    std::fs::write(tmp.join("config.toml"), "# Application config\nname = \"my-app\"\nversion = \"1.0.0\"\n\n[database]\nhost = \"localhost\"\nport = 5432\n\n[logging]\nlevel = \"info\"\nformat = \"json\"\n").unwrap();

    let output = Command::new("python3")
        .args([
            "-c",
            "import tomllib; tomllib.load(open('config.toml','rb')); print('OK')",
        ])
        .current_dir(&tmp)
        .output()
        .expect("python3 不可用");

    assert!(
        output.status.success(),
        "有效 TOML 应通过解析: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_toml_samples_are_detected() {
    let tmp = ensure_clean_dir("toml-invalid-test");
    std::fs::write(
        tmp.join("config.toml"),
        "name = \"my-app\"\nversion = 1.0.0\n[database\nhost = \"localhost\"\n",
    )
    .unwrap();

    let output = Command::new("python3")
        .args([
            "-c",
            "import tomllib; tomllib.load(open('config.toml','rb')); print('OK')",
        ])
        .current_dir(&tmp)
        .output()
        .expect("python3 不可用");

    assert!(!output.status.success(), "语法错误应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("toml-comment-test");
    // 格式正确但缺少注释——TOML 解析器不检查注释
    std::fs::write(tmp.join("config.toml"), "name = \"my-app\"\nversion = \"1.0.0\"\n\n[server]\nhost = \"0.0.0.0\"\nport = 8080\n\n[features]\nauth = true\ncache = false\n").unwrap();

    let output = Command::new("python3")
        .args([
            "-c",
            "import tomllib; tomllib.load(open('config.toml','rb')); print('OK')",
        ])
        .current_dir(&tmp)
        .output()
        .expect("python3 不可用");

    assert!(
        output.status.success(),
        "TOML 应通过（语法正确），但注释合规需另查: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}
