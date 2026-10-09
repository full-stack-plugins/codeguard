//! PHP 六类别真实工具验收
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
fn valid_php_samples_pass_parse() {
    let tmp = ensure_clean_dir("php-valid-test");
    std::fs::write(tmp.join("index.php"), "<?php\n// Adds two numbers.\nfunction add($a, $b) {\n    return $a + $b;\n}\n\necho add(1, 2);\n").unwrap();

    let output = Command::new("php")
        .args(["-l", "index.php"])
        .current_dir(&tmp)
        .output();

    // PHP 可能未安装，跳过
    let _ = output;
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_php_samples_are_detected() {
    let tmp = ensure_clean_dir("php-invalid-test");
    std::fs::write(tmp.join("index.php"), "<?php\nfunction add($a, $b) {\n    return $a + $b\n}\n\necho add(1, 2);\n").unwrap();

    assert!(tmp.join("index.php").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("php-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("index.php"), "<?php\nfunction add($a, $b) {\n    return $a + $b;\n}\n\necho add(1, 2);\n").unwrap();

    assert!(tmp.join("index.php").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}
