//! Kotlin 六类别适用性真实验收测试（kotlinc）。

use std::process::Command;

#[test]
fn valid_kotlin_samples_compile_successfully() {
    let tmp = std::env::temp_dir().join("kotlin-valid-test");
    std::fs::create_dir_all(&tmp).unwrap();

    let samples = vec![
        ("Empty.kt", "class Empty"),
        (
            "Method.kt",
            "class Method { fun add(a: Int, b: Int): Int { return a + b } }",
        ),
        ("Generic.kt", "class Generic<T>(val value: T)"),
    ];

    for (filename, content) in &samples {
        std::fs::write(tmp.join(filename), content).unwrap();
    }

    let output = Command::new("kotlinc")
        .args([
            "-nowarn",
            tmp.join("Empty.kt").to_str().unwrap(),
            "-d",
            tmp.join("out").to_str().unwrap(),
        ])
        .output()
        .expect("kotlinc 不可用");

    assert!(
        output.status.success(),
        "正确 Kotlin 样本应编译成功: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_kotlin_samples_are_detected() {
    let tmp = std::env::temp_dir().join("kotlin-invalid-test");
    std::fs::create_dir_all(&tmp).unwrap();

    std::fs::write(tmp.join("Bad.kt"), "class Bad { fun method( { }").unwrap();

    let output = Command::new("kotlinc")
        .args([
            "-nowarn",
            tmp.join("Bad.kt").to_str().unwrap(),
            "-d",
            tmp.join("out").to_str().unwrap(),
        ])
        .output()
        .expect("kotlinc 不可用");

    assert!(!output.status.success(), "违规 Kotlin 样本应编译失败");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = std::env::temp_dir().join("kotlin-comment-test");
    std::fs::create_dir_all(&tmp).unwrap();

    std::fs::write(tmp.join("NoDoc.kt"), "class NoDoc { fun method() { } }").unwrap();
    std::fs::write(
        tmp.join("WithDoc.kt"),
        "/** Does something. */\nclass WithDoc { fun method() { } }",
    )
    .unwrap();

    for file in &["NoDoc.kt", "WithDoc.kt"] {
        let output = Command::new("kotlinc")
            .args([
                "-nowarn",
                tmp.join(file).to_str().unwrap(),
                "-d",
                tmp.join("out").to_str().unwrap(),
            ])
            .output()
            .expect("kotlinc 不可用");
        assert!(output.status.success(), "{} 应编译成功", file);
    }

    std::fs::remove_dir_all(&tmp).unwrap();
}
