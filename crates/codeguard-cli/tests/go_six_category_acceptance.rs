//! Go 六类别适用性真实验收测试（go vet）。

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn isolated_project() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "codeguard-go-acceptance-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    root
}

#[test]
fn valid_go_samples_pass_vet() {
    let tmp = isolated_project();
    std::fs::write(tmp.join("go.mod"), "module example\ngo 1.21\n").unwrap();
    std::fs::write(tmp.join("main.go"), "package main\n\nfunc main() {}\n").unwrap();

    let output = Command::new("go")
        .args(["vet", "./..."])
        .current_dir(&tmp)
        .output()
        .expect("go 不可用");

    assert!(
        output.status.success(),
        "正确 Go 样本应通过 vet: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_go_samples_are_detected() {
    let tmp = isolated_project();
    std::fs::write(tmp.join("go.mod"), "module example\ngo 1.21\n").unwrap();
    // 语法错误
    std::fs::write(tmp.join("main.go"), "package main\n\nfunc main( { }\n").unwrap();

    let output = Command::new("go")
        .args(["vet", "./..."])
        .current_dir(&tmp)
        .output()
        .expect("go 不可用");

    assert!(!output.status.success(), "违规 Go 样本应被 vet 检出");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("main.go:3:"),
        "must observe a source diagnostic, not an environment failure: {output:?}"
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    // 关键验证：go vet 能通过（格式正确）不等于注释合规
    // go vet 检查的是代码正确性，不是文档完整性
    let tmp = isolated_project();
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
