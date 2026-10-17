//! Python 六类别适用性真实验收测试（python3）。

use std::process::Command;

#[test]
fn valid_python_samples_compile_successfully() {
    let tmp = std::env::temp_dir().join("python-valid-test");
    std::fs::create_dir_all(&tmp).unwrap();

    let samples = vec![
        ("empty.py", "class Empty:\n    pass\n"),
        (
            "method.py",
            "class Method:\n    def add(self, a, b):\n        return a + b\n",
        ),
        ("func.py", "def hello():\n    return 'hello'\n"),
    ];

    for (filename, content) in &samples {
        std::fs::write(tmp.join(filename), content).unwrap();
    }

    let output = Command::new("python3")
        .args(["-m", "py_compile", tmp.join("empty.py").to_str().unwrap()])
        .output()
        .expect("python3 不可用");

    assert!(
        output.status.success(),
        "正确 Python 样本应编译成功: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_python_samples_are_detected() {
    let tmp = std::env::temp_dir().join("python-invalid-test");
    std::fs::create_dir_all(&tmp).unwrap();

    std::fs::write(tmp.join("Bad.py"), "def method(\n    pass\n").unwrap();

    let output = Command::new("python3")
        .args(["-m", "py_compile", tmp.join("Bad.py").to_str().unwrap()])
        .output()
        .expect("python3 不可用");

    assert!(!output.status.success(), "违规 Python 样本应编译失败");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = std::env::temp_dir().join("python-comment-test");
    std::fs::create_dir_all(&tmp).unwrap();

    // 无 docstring
    std::fs::write(tmp.join("nodoc.py"), "def method():\n    pass\n").unwrap();
    // 有 docstring
    std::fs::write(
        tmp.join("withdoc.py"),
        "def method():\n    \"\"\"Does something.\"\"\"\n    pass\n",
    )
    .unwrap();

    for file in &["nodoc.py", "withdoc.py"] {
        let output = Command::new("python3")
            .args(["-m", "py_compile", tmp.join(file).to_str().unwrap()])
            .output()
            .expect("python3 不可用");
        assert!(output.status.success(), "{} 应编译成功", file);
    }

    std::fs::remove_dir_all(&tmp).unwrap();
}
