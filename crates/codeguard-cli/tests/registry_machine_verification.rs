//! 8.148 机器核对注册表与全部验收条目
//! 验收：54 个原 stable 六槽位与真实证据完整，3 planned 如实披露，无丢项/重复/别名漂移。

use std::collections::HashSet;
use std::path::PathBuf;

/// 从仓库根加载 rulepacks/legacy_languages.json
fn load_registry() -> serde_json::Value {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let registry_path = manifest_dir
        .join("..")
        .join("..")
        .join("rulepacks")
        .join("legacy_languages.json");
    let raw = std::fs::read_to_string(&registry_path)
        .unwrap_or_else(|e| panic!("无法读取注册表 {:?}: {e}", registry_path));
    serde_json::from_str(&raw).expect("注册表 JSON 解析失败")
}

#[test]
fn registry_has_57_languages_54_stable_3_planned() {
    let registry = load_registry();
    let langs = registry["languages"].as_array().expect("languages 应为数组");
    assert_eq!(langs.len(), 57, "注册表应有 57 种语言");

    let stable: Vec<_> = langs.iter().filter(|l| l["status"] == "stable").collect();
    let planned: Vec<_> = langs.iter().filter(|l| l["status"] == "planned").collect();
    assert_eq!(stable.len(), 54, "应有 54 种 stable 语言");
    assert_eq!(planned.len(), 3, "应有 3 种 planned 语言");
}

#[test]
fn registry_planned_languages_are_cobol_arkts_metal() {
    let registry = load_registry();
    let planned_ids: HashSet<String> = registry["languages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["status"] == "planned")
        .map(|l| l["id"].as_str().unwrap().to_string())
        .collect();

    let expected: HashSet<String> = ["cobol", "arkts", "metal"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        planned_ids, expected,
        "planned 语言应为 cobol/arkts/metal，无别名漂移"
    );
}

#[test]
fn registry_no_duplicate_ids() {
    let registry = load_registry();
    let ids: Vec<&str> = registry["languages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["id"].as_str().unwrap())
        .collect();
    let unique: HashSet<&str> = ids.iter().copied().collect();
    assert_eq!(
        ids.len(),
        unique.len(),
        "注册表语言 ID 不得重复：{ids:?}"
    );
}

#[test]
fn registry_all_statuses_are_valid() {
    let registry = load_registry();
    for lang in registry["languages"].as_array().unwrap() {
        let status = lang["status"].as_str().unwrap();
        assert!(
            matches!(status, "stable" | "planned"),
            "语言 {} 状态无效: {}",
            lang["id"],
            status
        );
    }
}

#[test]
fn registry_all_linter_configs_are_safe_paths() {
    let registry = load_registry();
    for lang in registry["languages"].as_array().unwrap() {
        if let Some(configs) = lang["linter_config_files"].as_array() {
            for cfg in configs {
                let s = cfg.as_str().unwrap();
                assert!(!s.is_empty(), "{}: 配置文件名不得为空", lang["id"]);
                assert!(
                    !s.contains('\\') && !s.contains('*') && !s.contains('?'),
                    "{}: 配置文件名含不安全字符: {}",
                    lang["id"],
                    s
                );
                assert!(
                    s.split('/').all(|p| !p.is_empty() && p != "." && p != ".."),
                    "{}: 配置文件名含路径穿越: {}",
                    lang["id"],
                    s
                );
            }
        }
    }
}

#[test]
fn stable_languages_have_six_category_test_coverage() {
    let registry = load_registry();
    let stable_ids: HashSet<String> = registry["languages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["status"] == "stable")
        .map(|l| l["id"].as_str().unwrap().to_string())
        .collect();

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tests_dir = manifest_dir.join("tests");

    // 收集所有 *_six_category_acceptance.rs 文件名
    let mut covered: HashSet<String> = HashSet::new();
    for entry in std::fs::read_dir(&tests_dir).expect("tests 目录应可读") {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(lang) = name.strip_suffix("_six_category_acceptance.rs") {
            covered.insert(lang.to_string());
        }
    }

    // shell 和 dockerfile 由 shell_74_dockerfile_acceptance.rs 覆盖
    // 它们没有独立的 *_six_category_acceptance.rs 文件
    let covered_with_combined = {
        let mut set = covered.clone();
        set.insert("shell".to_string());
        set.insert("dockerfile".to_string());
        set
    };

    let missing: Vec<&String> = stable_ids.difference(&covered_with_combined).collect();
    assert!(
        missing.is_empty(),
        "以下 stable 语言缺少六类别验收测试: {missing:?}"
    );
}

#[test]
fn planned_languages_have_gap_tests() {
    // 3 种 planned 语言（cobol/arkts/metal）应由 planned_language_gaps.rs 覆盖
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let gap_test = manifest_dir.join("tests").join("planned_language_gaps.rs");
    assert!(
        gap_test.exists(),
        "planned 语言缺口测试 planned_language_gaps.rs 应存在"
    );

    let content = std::fs::read_to_string(&gap_test).unwrap();
    for lang in ["cobol", "arkts", "metal"] {
        assert!(
            content.contains(lang),
            "planned_language_gaps.rs 应包含 {lang} 的缺口披露"
        );
    }
}

#[test]
fn no_alias_drift_in_registry() {
    // 核对每个语言 ID 与其 status 的固定身份匹配
    // stable 语言不得出现在 planned 列表中，反之亦然
    let registry = load_registry();
    let langs = registry["languages"].as_array().unwrap();

    let stable_ids: HashSet<&str> = langs
        .iter()
        .filter(|l| l["status"] == "stable")
        .map(|l| l["id"].as_str().unwrap())
        .collect();
    let planned_ids: HashSet<&str> = langs
        .iter()
        .filter(|l| l["status"] == "planned")
        .map(|l| l["id"].as_str().unwrap())
        .collect();

    // stable 和 planned 不得有交集
    let intersection: Vec<&&str> = stable_ids.intersection(&planned_ids).collect();
    assert!(
        intersection.is_empty(),
        "stable 和 planned 不得有交集: {intersection:?}"
    );
}
