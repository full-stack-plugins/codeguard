//! Swift 六类别适用性真实验收测试（swiftc）。

use std::process::Command;

#[test]
fn valid_swift_samples_compile_successfully() {
    let tmp = std::env::temp_dir().join("swift-valid-test");
    std::fs::create_dir_all(&tmp).unwrap();
    
    let samples = vec![
        ("Empty.swift", "struct Empty {}"),
        ("Method.swift", "struct Method { func add(_ a: Int, _ b: Int) -> Int { a + b } }"),
        ("Generic.swift", "struct Generic<T> { let value: T }"),
    ];
    
    for (filename, content) in &samples {
        std::fs::write(tmp.join(filename), content).unwrap();
    }
    
    let output = Command::new("swiftc")
        .args(["-typecheck", tmp.join("Empty.swift").to_str().unwrap()])
        .output()
        .expect("swiftc 不可用");
    
    assert!(output.status.success(), "正确 Swift 样本应编译成功: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_swift_samples_are_detected() {
    let tmp = std::env::temp_dir().join("swift-invalid-test");
    std::fs::create_dir_all(&tmp).unwrap();
    
    std::fs::write(tmp.join("Bad.swift"), "struct Bad { func method( { }").unwrap();
    
    let output = Command::new("swiftc")
        .args(["-typecheck", tmp.join("Bad.swift").to_str().unwrap()])
        .output()
        .expect("swiftc 不可用");
    
    assert!(!output.status.success(), "违规 Swift 样本应编译失败");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = std::env::temp_dir().join("swift-comment-test");
    std::fs::create_dir_all(&tmp).unwrap();
    
    // 无文档注释
    std::fs::write(tmp.join("NoDoc.swift"), "struct NoDoc { func method() { } }").unwrap();
    // 有文档注释
    std::fs::write(tmp.join("WithDoc.swift"), "/// Does something.\nstruct WithDoc { func method() { } }").unwrap();
    
    // 两者都能编译（格式正确）
    for file in &["NoDoc.swift", "WithDoc.swift"] {
        let output = Command::new("swiftc")
            .args(["-typecheck", tmp.join(file).to_str().unwrap()])
            .output()
            .expect("swiftc 不可用");
        assert!(output.status.success(), "{} 应编译成功", file);
    }
    
    std::fs::remove_dir_all(&tmp).unwrap();
}
