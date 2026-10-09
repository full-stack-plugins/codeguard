use std::process::Command;

fn ensure_clean_dir(name: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    tmp
}

#[test]
fn valid_perl_samples_pass_syntax_check() {
    let tmp = ensure_clean_dir("perl-valid-test");
    std::fs::write(tmp.join("example.pl"), "#!/usr/bin/perl\nuse strict;\nuse warnings;\n\n# Adds two numbers.\nsub add {\n    my ($a, $b) = @_;\n    return $a + $b;\n}\n\nprint add(1, 2), \"\\n\";\n").unwrap();

    let output = Command::new("perl")
        .args(["-c", "example.pl"])
        .current_dir(&tmp)
        .output()
        .expect("perl 不可用");

    assert!(
        output.status.success(),
        "有效 Perl 代码应通过语法检查: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn invalid_perl_samples_are_detected() {
    let tmp = ensure_clean_dir("perl-invalid-test");
    std::fs::write(tmp.join("example.pl"), "#!/usr/bin/perl\nuse strict;\nuse warnings;\n\nsub broken {\n    my $x = ;  # syntax error\n    return $x;\n}\n").unwrap();

    let output = Command::new("perl")
        .args(["-c", "example.pl"])
        .current_dir(&tmp)
        .output()
        .expect("perl 不可用");

    assert!(!output.status.success(), "语法错误应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("perl-comment-test");
    // 格式正确但缺少 POD 文档——perl -c 不检查文档
    std::fs::write(tmp.join("example.pl"), "#!/usr/bin/perl\nuse strict;\nuse warnings;\n\nsub undocumented {\n    my ($a) = @_;\n    return $a * 2;\n}\n\n# Has docs.\nsub documented {\n    my ($a) = @_;\n    return $a + 1;\n}\n\nprint undocumented(5) + documented(3), \"\\n\";\n").unwrap();

    let output = Command::new("perl")
        .args(["-c", "example.pl"])
        .current_dir(&tmp)
        .output()
        .expect("perl 不可用");

    assert!(
        output.status.success(),
        "perl 应通过（语法正确），但文档合规需另查: {:?}",
        output
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}
