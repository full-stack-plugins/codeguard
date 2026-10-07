use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_zig_samples_pass_build() {
    let tmp = ensure_clean_dir("zig-valid-test");
    std::fs::write(tmp.join("main.zig"), "const std = @import(\"std\");\n\n/// Adds two integers.\nfn add(a: i32, b: i32) i32 {\n    return a + b;\n}\n\npub fn main() void {\n    std.debug.print(\"{}\\n\", .{add(1, 2)});\n}\n").unwrap();

    let output = Command::new("zig")
        .args([
            "build-exe",
            "main.zig",
            "--cache-dir",
            ".zig-cache",
            "--global-cache-dir",
            ".zig-global-cache",
        ])
        .current_dir(&tmp)
        .output()
        .expect("zig 不可用");

    assert!(
        output.status.success(),
        "有效 Zig 代码应通过编译: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_zig_samples_are_detected() {
    let tmp = ensure_clean_dir("zig-invalid-test");
    std::fs::write(tmp.join("main.zig"), "const std = @import(\"std\");\n\npub fn main() void {\n    const x: i32 = \"not a number\";\n    _ = x;\n}\n").unwrap();

    let output = Command::new("zig")
        .args([
            "build-exe",
            "main.zig",
            "--cache-dir",
            ".zig-cache",
            "--global-cache-dir",
            ".zig-global-cache",
        ])
        .current_dir(&tmp)
        .output()
        .expect("zig 不可用");

    assert!(!output.status.success(), "类型错误应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("zig-comment-test");
    // 格式正确但缺少文档——编译器不检查文档
    std::fs::write(tmp.join("main.zig"), "const std = @import(\"std\");\n\nfn undocumented(a: i32) i32 {\n    return a * 2;\n}\n\n/// Has docs.\nfn documented(a: i32) i32 {\n    return a + 1;\n}\n\npub fn main() void {\n    std.debug.print(\"{}\\n\", .{undocumented(5) + documented(3)});\n}\n").unwrap();

    let output = Command::new("zig")
        .args([
            "build-exe",
            "main.zig",
            "--cache-dir",
            ".zig-cache",
            "--global-cache-dir",
            ".zig-global-cache",
        ])
        .current_dir(&tmp)
        .output()
        .expect("zig 不可用");

    assert!(
        output.status.success(),
        "zig 应通过（代码正确），但文档合规需另查: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}
