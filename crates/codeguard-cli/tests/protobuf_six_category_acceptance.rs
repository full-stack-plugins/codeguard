use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_protobuf_samples_pass_compile() {
    let tmp = ensure_clean_dir("protobuf-valid-test");
    std::fs::write(tmp.join("example.proto"), "syntax = \"proto3\";\n\npackage example;\n\n// User represents a user.\nmessage User {\n    string name = 1;\n    int32 age = 2;\n    string email = 3;\n}\n").unwrap();

    let output = Command::new("protoc")
        .args(["--descriptor_set_out=/dev/null", "example.proto"])
        .current_dir(&tmp)
        .output()
        .expect("protoc 不可用");

    assert!(output.status.success(), "有效 Protobuf 应通过编译: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_protobuf_samples_are_detected() {
    let tmp = ensure_clean_dir("protobuf-invalid-test");
    std::fs::write(tmp.join("example.proto"), "syntax = \"proto3\";\n\npackage example;\n\nmessage User {\n    string name = 1;\n    int32 age = ;  // missing field number\n    string email = 3;\n}\n").unwrap();

    let output = Command::new("protoc")
        .args(["--descriptor_set_out=/dev/null", "example.proto"])
        .current_dir(&tmp)
        .output()
        .expect("protoc 不可用");

    assert!(!output.status.success(), "语法错误应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("protobuf-comment-test");
    // 格式正确但缺少注释——protoc 不检查注释
    std::fs::write(tmp.join("example.proto"), "syntax = \"proto3\";\n\npackage example;\n\nmessage Product {\n    string name = 1;\n    double price = 2;\n}\n\nmessage Order {\n    int32 id = 1;\n    string product_name = 2;\n}\n").unwrap();

    let output = Command::new("protoc")
        .args(["--descriptor_set_out=/dev/null", "example.proto"])
        .current_dir(&tmp)
        .output()
        .expect("protoc 不可用");

    assert!(output.status.success(), "Protobuf 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
