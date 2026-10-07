//! Go 六类别适用性真实验收测试（go vet）。

use std::process::Command;

#[test]
fn valid_go_samples_pass_vet() {
    let tmp = std::env::temp_dir().join("go-comment-test");
    let _ = std::fs::remove_dir_all(&tmp);
    let tmp = std::env::temp_dir().join("go-valid-test");
    std::fs::create_dir_all(tmp.clone()).unwrap();
    std::fs::write(tmp.join("go.mod"), "module example\ngo 1.21\n").unwrap();
    std::fs::write(tmp.join("main.go"), "package main\n\nfunc main() {}\n").unwrap();
    
    let output = Command::new("go")
        .args(["vet", "./..."])
        .current_dir(&tmp)
        .output()
        .expect("go 不可用");
    
    assert!(output.status.success(), "正确 Go 样本应通过 vet: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_go_samples_are_detected() {
    let tmp = std::env::temp_dir().join("go-comment-test");
    let _ = std::fs::remove_dir_all(&tmp);
    let tmp = std::env::temp_dir().join("go-invalid-test");
    std::fs::create_dir_all(tmp.clone()).unwrap();
    std::fs::write(tmp.join("go.mod"), "module example\ngo 1.21\n").unwrap();
    // 语法错误
    std::fs::write(tmp.join("main.go"), "package main\n\nfunc main( { }\n").unwrap();
    
    let output = Command::new("go")
        .args(["vet", "./..."])
        .current_dir(&tmp)
        .output()
        .expect("go 不可用");
    
    assert!(!output.status.success(), "违规 Go 样本应被 vet 检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = std::env::temp_dir().join("go-comment-test");
    let _ = std::fs::remove_dir_all(&tmp);
    // 关键验证：go vet 能通过（格式正确）不等于注释合规
    // go vet 检查的是代码正确性，不是文档完整性
    let tmp = std::env::temp_dir().join("go-comment-test");
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(tmp.join("go.mod"), "module example\ngo 1.21\n").unwrap();
    std::fs::write(
        tmp.join("main.go"),
        "package main\n\n// NoDoc is undocumented.\nfunc NoDoc() {}\n\n// WithDoc does something.\nfunc WithDoc() {}\n\nfunc main() {}\n",
    )
    .unwrap();

    let output = Command::new("go")
        .args(["vet", "./..."])
        .current_dir(&tmp)
        .output()
        .expect("go 不可用");

    // go vet 只检查代码正确性，不检查文档完整性
    // 所以格式正确（vet 通过）≠ 注释合规
    assert!(
        output.status.success(),
        "go vet 应通过（格式正确），但注释合规需另查: {:?}",
        output
    );

    std::fs::remove_dir_all(&tmp).unwrap();
}
