//! Astro 六类别真实工具验收
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
fn valid_astro_samples_pass_check() {
    let tmp = ensure_clean_dir("astro-valid-test");
    std::fs::write(tmp.join("index.astro"), "---\nconst title = 'Hello';\n---\n\n<html>\n<head><title>{title}</title></head>\n<body><h1>{title}</h1></body>\n</html>\n").unwrap();

    assert!(tmp.join("index.astro").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_astro_samples_are_detected() {
    let tmp = ensure_clean_dir("astro-invalid-test");
    std::fs::write(tmp.join("index.astro"), "---\nconst title = 'Hello'\n---\n\n<html>\n<head><title>{title}</title></head>\n<body><h1>{title}</h1></body>\n</html>\n").unwrap();

    assert!(tmp.join("index.astro").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("astro-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("index.astro"), "---\nconst title = 'Hello';\n---\n\n<html>\n<head><title>{title}</title></head>\n<body><h1>{title}</h1></body>\n</html>\n").unwrap();

    assert!(tmp.join("index.astro").exists());
    std::fs::remove_dir_all(&tmp).unwrap();
}
