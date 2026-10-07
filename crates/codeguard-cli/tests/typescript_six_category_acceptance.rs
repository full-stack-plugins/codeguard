use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_typescript_samples_pass_check() {
    let tmp = ensure_clean_dir("ts-valid-test");
    std::fs::write(tmp.join("tsconfig.json"), "{\n  \"compilerOptions\": {\n    \"strict\": true,\n    \"noEmit\": true\n  }\n}\n").unwrap();
    std::fs::write(tmp.join("main.ts"), "/** Adds two numbers. */\nfunction add(a: number, b: number): number {\n  return a + b;\n}\n\nconsole.log(add(1, 2));\n").unwrap();

    let _ = Command::new("npm")
        .args(["install", "typescript", "--no-save"])
        .current_dir(&tmp)
        .output()
        .expect("npm 不可用");
    let output = Command::new("npx")
        .args(["tsc", "--noEmit", "--project", "."])
        .current_dir(&tmp)
        .output()
        .expect("tsc 不可用");

    assert!(output.status.success(), "有效 TS 代码应通过 tsc: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_typescript_samples_are_detected() {
    let tmp = ensure_clean_dir("ts-invalid-test");
    std::fs::write(tmp.join("tsconfig.json"), "{\n  \"compilerOptions\": {\n    \"strict\": true,\n    \"noEmit\": true\n  }\n}\n").unwrap();
    std::fs::write(tmp.join("main.ts"), "function broken(a: number): string {\n  return a; // type error: returning number from string function\n}\n").unwrap();

    let _ = Command::new("npm")
        .args(["install", "typescript", "--no-save"])
        .current_dir(&tmp)
        .output()
        .expect("npm 不可用");
    let output = Command::new("npx")
        .args(["tsc", "--noEmit", "--project", "."])
        .current_dir(&tmp)
        .output()
        .expect("tsc 不可用");

    assert!(!output.status.success(), "类型错误应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("ts-comment-test");
    std::fs::write(tmp.join("tsconfig.json"), "{\n  \"compilerOptions\": {\n    \"strict\": true,\n    \"noEmit\": true\n  }\n}\n").unwrap();
    // 格式正确但缺少文档——tsc 不检查文档
    std::fs::write(tmp.join("main.ts"), "function undocumented(a: number): number {\n  return a * 2;\n}\n\n/** Has docs. */\nfunction documented(a: number): number {\n  return a + 1;\n}\n\nconsole.log(undocumented(5) + documented(3));\n").unwrap();

    let _ = Command::new("npm")
        .args(["install", "typescript", "--no-save"])
        .current_dir(&tmp)
        .output()
        .expect("npm 不可用");
    let output = Command::new("npx")
        .args(["tsc", "--noEmit", "--project", "."])
        .current_dir(&tmp)
        .output()
        .expect("tsc 不可用");

    assert!(output.status.success(), "tsc 应通过（类型正确），但文档合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
