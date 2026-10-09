//! IDEA markdown 内置格式化器的 golden 差分测试。
//!
//! fixtures 下每个 `X.md` 配对 `X.expected.md`：后者是 IntelliJ IDEA
//! 2026.2.3 `format.sh -allowDefaults` 的**真实输出**（在装有该版本的
//! 机器上生成后提交入库）。CI 无需安装 IDEA——一致性被 golden 锁定。
//!
//! 差分失败的两种含义：
//! 1. 内置实现偏离 IDEA 风格 → 修实现；
//! 2. IDEA 升级改变了风格 → 在装有新版本的机器上重新生成 golden，
//!    并同步提升 idea_markdown::RULES_VERSION。
//!
//! golden 覆盖的规则面（由探针样本确定）：
//! 标题空格/无空格标题/标题内空格保留、正文空格收拢、code span 保护、
//! 列表标记保留与块间距、同标记列表不插空行、嵌套列表、有序列表、
//! 表格 CJK 列宽对齐、引用、分隔线、空行收敛、尾部换行事实。

use codeguard_cli::idea_markdown;

fn fixtures() -> Vec<std::path::PathBuf> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/idea_markdown");
    let mut pairs = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("fixtures 目录应存在") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("md")
            && !path.to_string_lossy().ends_with(".expected.md")
        {
            pairs.push(path);
        }
    }
    pairs.sort();
    assert!(!pairs.is_empty(), "fixtures 不应为空");
    pairs
}

fn show_diff(label: &str, input: &str, expected: &str, actual: &str) {
    eprintln!("== {label} 差分 ==");
    let exp: Vec<&str> = expected.lines().collect();
    let act: Vec<&str> = actual.lines().collect();
    for i in 0..exp.len().max(act.len()) {
        let e = exp.get(i).copied().unwrap_or("<缺>");
        let a = act.get(i).copied().unwrap_or("<缺>");
        if e != a {
            eprintln!("  行{} 期望: {:?}\n      实际: {:?}", i + 1, e, a);
        }
    }
    let _ = input;
}

/// 内置实现必须与 IDEA 真实输出逐字节一致。
#[test]
fn builtin_matches_idea_golden_byte_for_byte() {
    for input_path in fixtures() {
        let label = input_path.file_name().unwrap().to_string_lossy().to_string();
        let input = std::fs::read_to_string(&input_path).expect("读入样本");
        // 命名约定 X.md → X.expected.md
        let expected_path = input_path
            .parent()
            .unwrap()
            .join(format!("{}.expected.md", input_path.file_stem().unwrap().to_string_lossy()));
        let expected = std::fs::read_to_string(&expected_path)
            .unwrap_or_else(|e| panic!("缺配对 golden {expected_path:?}: {e}"));

        let actual = idea_markdown::apply(&input);
        assert_eq!(
            actual,
            expected,
            "{label} 与 IDEA golden 不一致（规则版本 {}）",
            idea_markdown::RULES_VERSION
        );
    }
}

/// 差分双向闭环：内置 apply 后必须稳定（幂等），
/// 且 check 对原始样本报不合规、对 golden 报合规。
#[test]
fn builtin_check_apply_roundtrip_on_golden() {
    for input_path in fixtures() {
        let input = std::fs::read_to_string(&input_path).expect("读入样本");
        let formatted = idea_markdown::apply(&input);

        // 幂等：格式化产物再格式化不变
        assert_eq!(
            idea_markdown::apply(&formatted),
            formatted,
            "{} 格式化不幂等——会产生抖动（每次 check 都报不合规）",
            input_path.display()
        );

        // check 三态
        let (status_clean, _) = idea_markdown::check(&formatted);
        assert_eq!(status_clean, "clean", "格式化产物应判 clean");
        if formatted != input {
            let (status_dirty, _) = idea_markdown::check(&input);
            assert_eq!(status_dirty, "unformatted", "原始样本应判 unformatted");
        }
    }
}
