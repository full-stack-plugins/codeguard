use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_rust_samples_pass_cargo_check() {
    let tmp = ensure_clean_dir("rust-valid-test");
    std::fs::write(
        tmp.join("Cargo.toml"),
        "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(tmp.join("src")).unwrap();
    std::fs::write(tmp.join("src/main.rs"), "/// Adds two numbers.\nfn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\nfn main() {\n    println!(\"{}\", add(1, 2));\n}\n").unwrap();

    let output = Command::new("cargo")
        .args(["check", "--offline"])
        .current_dir(&tmp)
        .output()
        .expect("cargo 不可用");

    assert!(
        output.status.success(),
        "有效 Rust 代码应通过 cargo check: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_rust_samples_are_detected() {
    let tmp = ensure_clean_dir("rust-invalid-test");
    std::fs::write(
        tmp.join("Cargo.toml"),
        "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(tmp.join("src")).unwrap();
    std::fs::write(
        tmp.join("src/main.rs"),
        "fn main() {\n    let x: i32 = \"not a number\";\n    println!(\"{}\", x);\n}\n",
    )
    .unwrap();

    let output = Command::new("cargo")
        .args(["check", "--offline"])
        .current_dir(&tmp)
        .output()
        .expect("cargo 不可用");

    assert!(!output.status.success(), "类型错误应被检出");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("mismatched types") || stderr.contains("expected"),
        "应报告类型错误: {}",
        stderr
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("rust-comment-test");
    std::fs::write(
        tmp.join("Cargo.toml"),
        "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(tmp.join("src")).unwrap();
    // 代码格式正确但缺少文档注释——cargo check 不检查文档
    std::fs::write(tmp.join("src/main.rs"), "fn undocumented_function() -> i32 {\n    42\n}\n\n/// This one has docs.\nfn documented_function() -> i32 {\n    0\n}\n\nfn main() {\n    println!(\"{}\", undocumented_function() + documented_function());\n}\n").unwrap();

    let output = Command::new("cargo")
        .args(["check", "--offline"])
        .current_dir(&tmp)
        .output()
        .expect("cargo 不可用");

    // cargo check 只检查代码正确性，不检查文档完整性
    assert!(
        output.status.success(),
        "cargo check 应通过（代码正确），但文档合规需另查: {:?}",
        output
    );

    std::fs::remove_dir_all(&tmp).unwrap();
}
