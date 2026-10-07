//! Clojure 六类别真实工具验收
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
fn valid_clojure_samples_pass_syntax() {
    let tmp = ensure_clean_dir("clojure-valid-test");
    std::fs::write(tmp.join("main.clj"), ";; Adds two numbers.\n(defn add [a b]\n  (+ a b))\n\n(println (add 1 2))\n").unwrap();

    let output = Command::new("clojure")
        .args(["-M", "-e", "(load-file \"main.clj\")"])
        .current_dir(&tmp)
        .output()
        .expect("clojure 不可用");

    assert!(output.status.success(), "有效 Clojure 代码应通过执行: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 负例检出
#[test]
fn invalid_clojure_samples_are_detected() {
    let tmp = ensure_clean_dir("clojure-invalid-test");
    std::fs::write(tmp.join("main.clj"), "(defn broken [a b]\n  (+ a b\n").unwrap();

    let output = Command::new("clojure")
        .args(["-M", "-e", "(load-file \"main.clj\")"])
        .current_dir(&tmp)
        .output()
        .expect("clojure 不可用");

    assert!(!output.status.success(), "语法错误应被检出");
    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 格式≠注释合规
#[test]
fn formatting_cannot_impersonate_comment_checking() {
    let tmp = ensure_clean_dir("clojure-comment-test");
    // 格式正确但缺少注释
    std::fs::write(tmp.join("main.clj"), "(defn undocumented [a]\n  (* a 2))\n\n;; Has docs.\n(defn documented [a]\n  (+ a 1))\n\n(println (+ (undocumented 5) (documented 3)))\n").unwrap();

    let output = Command::new("clojure")
        .args(["-M", "-e", "(load-file \"main.clj\")"])
        .current_dir(&tmp)
        .output()
        .expect("clojure 不可用");

    assert!(output.status.success(), "Clojure 应通过（语法正确），但注释合规需另查: {:?}", output);
    std::fs::remove_dir_all(&tmp).unwrap();
}
