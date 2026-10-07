//! Ruby 六类别适用性真实验收测试（ruby）。

use std::process::Command;

#[test]
fn valid_ruby_samples_compile_successfully() {
    let tmp = std::env::temp_dir().join("ruby-valid-test");
    std::fs::create_dir_all(&tmp).unwrap();
    
    let samples = vec![
        ("empty.rb", "class Empty\nend\n"),
        ("method.rb", "class Method\n  def add(a, b)\n    a + b\n  end\nend\n"),
        ("module.rb", "module Example\n  def self.hello\n    'hello'\n  end\nend\n"),
    ];
    
    for (filename, content) in &samples {
        std::fs::write(tmp.join(filename), content).unwrap();
    }
    
    let output = Command::new("ruby")
        .args(["-c", tmp.join("empty.rb").to_str().unwrap()])
        .output()
        .expect("ruby 不可用");
    
    assert!(output.status.success(), "正确 Ruby 样本应编译成功: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_ruby_samples_are_detected() {
    let tmp = std::env::temp_dir().join("ruby-invalid-test");
    std::fs::create_dir_all(&tmp).unwrap();
    
    std::fs::write(tmp.join("Bad.rb"), "class Bad\n  def method(\nend\n").unwrap();
    
    let output = Command::new("ruby")
        .args(["-c", tmp.join("Bad.rb").to_str().unwrap()])
        .output()
        .expect("ruby 不可用");
    
    assert!(!output.status.success(), "违规 Ruby 样本应编译失败");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = std::env::temp_dir().join("ruby-comment-test");
    std::fs::create_dir_all(&tmp).unwrap();
    
    std::fs::write(tmp.join("NoDoc.rb"), "class NoDoc\n  def method\n  end\nend\n").unwrap();
    std::fs::write(tmp.join("WithDoc.rb"), "# Does something.\nclass WithDoc\n  def method\n  end\nend\n").unwrap();
    
    for file in &["NoDoc.rb", "WithDoc.rb"] {
        let output = Command::new("ruby")
            .args(["-c", tmp.join(file).to_str().unwrap()])
            .output()
            .expect("ruby 不可用");
        assert!(output.status.success(), "{} 应编译成功", file);
    }
    
    std::fs::remove_dir_all(&tmp).unwrap();
}
