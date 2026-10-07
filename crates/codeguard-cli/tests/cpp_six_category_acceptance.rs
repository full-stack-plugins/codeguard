use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_cpp_samples_pass_compile() {
    let tmp = ensure_clean_dir("cpp-valid-test");
    std::fs::write(tmp.join("main.cpp"), "#include <iostream>\n\n/// Adds two integers.\nint add(int a, int b) {\n    return a + b;\n}\n\nint main() {\n    std::cout << add(1, 2) << std::endl;\n    return 0;\n}\n").unwrap();

    let output = Command::new("c++")
        .args([
            "-Wall",
            "-Wextra",
            "-std=c++17",
            "-c",
            "main.cpp",
            "-o",
            "main.o",
        ])
        .current_dir(&tmp)
        .output()
        .expect("c++ 不可用");

    assert!(
        output.status.success(),
        "有效 C++ 代码应通过编译: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_cpp_samples_are_detected() {
    let tmp = ensure_clean_dir("cpp-invalid-test");
    std::fs::write(
        tmp.join("main.cpp"),
        "int main() {\n    undefined_function();\n    return 0;\n}\n",
    )
    .unwrap();

    let output = Command::new("c++")
        .args([
            "-Wall",
            "-Wextra",
            "-std=c++17",
            "-c",
            "main.cpp",
            "-o",
            "main.o",
        ])
        .current_dir(&tmp)
        .output()
        .expect("c++ 不可用");

    assert!(!output.status.success(), "未声明函数应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("cpp-comment-test");
    std::fs::write(tmp.join("main.cpp"), "#include <iostream>\n\nint undocumented(int a) {\n    return a * 2;\n}\n\n/// Has docs.\nint documented(int a) {\n    return a + 1;\n}\n\nint main() {\n    std::cout << undocumented(5) + documented(3) << std::endl;\n    return 0;\n}\n").unwrap();

    let output = Command::new("c++")
        .args([
            "-Wall",
            "-Wextra",
            "-std=c++17",
            "-c",
            "main.cpp",
            "-o",
            "main.o",
        ])
        .current_dir(&tmp)
        .output()
        .expect("c++ 不可用");

    assert!(
        output.status.success(),
        "c++ 应通过（代码正确），但文档合规需另查: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}
