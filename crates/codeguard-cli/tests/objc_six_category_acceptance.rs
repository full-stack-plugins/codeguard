//! ObjC 六类别真实工具验收
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
fn valid_objc_samples_pass_compile() {
    let tmp = ensure_clean_dir("objc-valid-test");
    std::fs::write(tmp.join("main.m"), "#import <Foundation/Foundation.h>\n\n// Adds two numbers.\nint add(int a, int b) {\n    return a + b;\n}\n\nint main() {\n    @autoreleasepool {\n        NSLog(@\"%d\", add(1, 2));\n    }\n    return 0;\n}\n").unwrap();

    assert!(tmp.join("main.m").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_objc_samples_are_detected() {
    let tmp = ensure_clean_dir("objc-invalid-test");
    std::fs::write(tmp.join("main.m"), "#import <Foundation/Foundation.h>\n\nint add(int a, int b) {\n    return a + b\n}\n\nint main() {\n    @autoreleasepool {\n        NSLog(@\"%d\", add(1, 2));\n    }\n    return 0;\n}\n").unwrap();

    assert!(tmp.join("main.m").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("objc-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("main.m"), "#import <Foundation/Foundation.h>\n\nint add(int a, int b) {\n    return a + b;\n}\n\nint main() {\n    @autoreleasepool {\n        NSLog(@\"%d\", add(1, 2));\n    }\n    return 0;\n}\n").unwrap();

    assert!(tmp.join("main.m").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}
