use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_erlang_samples_pass_compile() {
    let tmp = ensure_clean_dir("erlang-valid-test");
    std::fs::write(
        tmp.join("example.erl"),
        "-module(example).\n-export([add/2]).\n\n%% @doc Adds two numbers.\nadd(A, B) -> A + B.\n",
    )
    .unwrap();

    let output = Command::new("erlc")
        .args(["example.erl"])
        .current_dir(&tmp)
        .output()
        .expect("erlc 不可用");

    assert!(
        output.status.success(),
        "有效 Erlang 代码应通过编译: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_erlang_samples_are_detected() {
    let tmp = ensure_clean_dir("erlang-invalid-test");
    std::fs::write(
        tmp.join("example.erl"),
        "-module(example).\n-export([add/2]).\n\nadd(A, B) -> A + B + Undefined.\n",
    )
    .unwrap();

    let output = Command::new("erlc")
        .args(["example.erl"])
        .current_dir(&tmp)
        .output()
        .expect("erlc 不可用");

    assert!(!output.status.success(), "未定义变量应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("erlang-comment-test");
    // 格式正确但缺少 @doc 文档——编译器不检查文档
    std::fs::write(tmp.join("example.erl"), "-module(example).\n-export([undocumented/1, documented/1]).\n\nundocumented(A) -> A * 2.\n\n%% @doc Has docs.\ndocumented(A) -> A + 1.\n").unwrap();

    let output = Command::new("erlc")
        .args(["example.erl"])
        .current_dir(&tmp)
        .output()
        .expect("erlc 不可用");

    assert!(
        output.status.success(),
        "erlc 应通过（代码正确），但文档合规需另查: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}
