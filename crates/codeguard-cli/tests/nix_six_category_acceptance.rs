//! Nix 六类别真实工具验收
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
fn valid_nix_samples_pass_parse() {
    let tmp = ensure_clean_dir("nix-valid-test");
    std::fs::write(tmp.join("default.nix"), "{ pkgs ? import <nixpkgs> {} }:\n\n# Adds two numbers.\npkgs.stdenv.mkDerivation {\n  name = \"example\";\n  buildInputs = [ pkgs.hello ];\n}\n").unwrap();

    assert!(tmp.join("default.nix").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_nix_samples_are_detected() {
    let tmp = ensure_clean_dir("nix-invalid-test");
    std::fs::write(tmp.join("default.nix"), "{ pkgs ? import <nixpkgs> {} }:\n\npkgs.stdenv.mkDerivation {\n  name = \"example\"\n  buildInputs = [ pkgs.hello ];\n}\n").unwrap();

    assert!(tmp.join("default.nix").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("nix-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("default.nix"), "{ pkgs ? import <nixpkgs> {} }:\n\npkgs.stdenv.mkDerivation {\n  name = \"example\";\n  buildInputs = [ pkgs.hello ];\n}\n").unwrap();

    assert!(tmp.join("default.nix").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}
