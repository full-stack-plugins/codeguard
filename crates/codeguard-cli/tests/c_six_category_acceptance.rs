use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_c_samples_pass_compile() {
    let tmp = ensure_clean_dir("c-valid-test");
    std::fs::write(tmp.join("main.c"), "#include <stdio.h>\n\n/** Adds two integers. */\nint add(int a, int b) {\n    return a + b;\n}\n\nint main(void) {\n    printf(\"%d\\n\", add(1, 2));\n    return 0;\n}\n").unwrap();

    let output = Command::new("cc")
        .args(["-Wall", "-Wextra", "-c", "main.c", "-o", "main.o"])
        .current_dir(&tmp)
        .output()
        .expect("cc 不可用");

    assert!(
        output.status.success(),
        "有效 C 代码应通过编译: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_c_samples_are_detected() {
    let tmp = ensure_clean_dir("c-invalid-test");
    std::fs::write(
        tmp.join("main.c"),
        "int main(void) {\n    undefined_function();\n    return 0;\n}\n",
    )
    .unwrap();

    let output = Command::new("cc")
        .args(["-Wall", "-Wextra", "-c", "main.c", "-o", "main.o"])
        .current_dir(&tmp)
        .output()
        .expect("cc 不可用");

    assert!(!output.status.success(), "未声明函数应被检出");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("implicit") || stderr.contains("undefined"),
        "应报告未声明: {}",
        stderr
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("c-comment-test");
    // 格式正确但缺少文档——编译器不检查文档
    std::fs::write(tmp.join("main.c"), "#include <stdio.h>\n\nint undocumented(int a) {\n    return a * 2;\n}\n\n/** Has docs. */\nint documented(int a) {\n    return a + 1;\n}\n\nint main(void) {\n    printf(\"%d\\n\", undocumented(5) + documented(3));\n    return 0;\n}\n").unwrap();

    let output = Command::new("cc")
        .args(["-Wall", "-Wextra", "-c", "main.c", "-o", "main.o"])
        .current_dir(&tmp)
        .output()
        .expect("cc 不可用");

    assert!(
        output.status.success(),
        "cc 应通过（代码正确），但文档合规需另查: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}
