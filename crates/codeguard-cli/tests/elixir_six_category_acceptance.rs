use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_elixir_samples_pass_compile() {
    let tmp = ensure_clean_dir("elixir-valid-test");
    std::fs::write(tmp.join("example.ex"), "defmodule Example do\n  @doc \"\"\"\n  Adds two numbers.\n  \"\"\"\n  def add(a, b) do\n    a + b\n  end\nend\n").unwrap();

    let output = Command::new("elixirc")
        .args(["example.ex"])
        .current_dir(&tmp)
        .output()
        .expect("elixir 不可用");

    assert!(output.status.success(), "有效 Elixir 代码应通过编译: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_elixir_samples_are_detected() {
    let tmp = ensure_clean_dir("elixir-invalid-test");
    std::fs::write(tmp.join("example.ex"), "defmodule Example do\n  def add(a, b) do\n    a + b + undefined_function()\n  end\nend\n").unwrap();

    let output = Command::new("elixirc")
        .args(["example.ex"])
        .current_dir(&tmp)
        .output()
        .expect("elixir 不可用");

    assert!(!output.status.success(), "未定义变量应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("elixir-comment-test");
    // 格式正确但缺少 @doc 文档——编译器不检查文档
    std::fs::write(tmp.join("example.ex"), "defmodule Example do\n  def undocumented(a) do\n    a * 2\n  end\n\n  @doc \"\"\"\n  Has docs.\n  \"\"\"\n  def documented(a) do\n    a + 1\n  end\nend\n").unwrap();

    let output = Command::new("elixirc")
        .args(["example.ex"])
        .current_dir(&tmp)
        .output()
        .expect("elixir 不可用");

    assert!(output.status.success(), "elixir 应通过（代码正确），但文档合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
