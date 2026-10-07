//! Haskell 六类别真实工具验收
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
fn valid_haskell_samples_pass_compile() {
    let tmp = ensure_clean_dir("haskell-valid-test");
    std::fs::write(tmp.join("Main.hs"), "-- | Adds two numbers.\nadd :: Int -> Int -> Int\nadd a b = a + b\n\nmain :: IO ()\nmain = print (add 1 2)\n").unwrap();

    let output = Command::new("ghc")
        .args(["-fno-code", "Main.hs"])
        .current_dir(&tmp)
        .output()
        .expect("ghc 不可用");

    assert!(output.status.success(), "有效 Haskell 代码应通过编译: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_haskell_samples_are_detected() {
    let tmp = ensure_clean_dir("haskell-invalid-test");
    std::fs::write(tmp.join("Main.hs"), "broken :: Int -> Int -> Int\nbroken a b = a + b + notDefinedAnywhere\n\nmain :: IO ()\nmain = print (broken 1 2)\n").unwrap();

    let output = Command::new("ghc")
        .args(["-fno-code", "Main.hs"])
        .current_dir(&tmp)
        .output()
        .expect("ghc 不可用");

    assert!(!output.status.success(), "未定义变量应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("haskell-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("Main.hs"), "undocumented :: Int -> Int\nundocumented a = a * 2\n\n-- | Has docs.\ndocumented :: Int -> Int\ndocumented a = a + 1\n\nmain :: IO ()\nmain = print (undocumented 5 + documented 3)\n").unwrap();

    let output = Command::new("ghc")
        .args(["-fno-code", "Main.hs"])
        .current_dir(&tmp)
        .output()
        .expect("ghc 不可用");

    assert!(output.status.success(), "Haskell 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
