//! 公共CLI语言别名；只改写明确的语言参数位置，不推测方言或文件路径。

/// 归一已登记的语言短名；参数为用户语言名，返回规范ID或原输入。
pub fn canonical_language(value: &str) -> &str {
    match value {
        "py" => "python",
        "rs" => "rust",
        "ts" => "typescript",
        "rb" => "ruby",
        "kt" => "kotlin",
        "erl" => "erlang",
        "golang" => "go",
        "c++" => "cpp",
        "c#" => "csharp",
        _ => value,
    }
}

/// 归一公共检查入口的语言位置；参数是CLI argv，不改写路径、工具或子命令。
pub fn normalize_arguments(args: &mut [String]) {
    let slot = match args.first().map(String::as_str) {
        Some("lint" | "comments" | "dependencies" | "cve" | "security" | "build" | "check") => 1,
        Some("plan") => 2,
        _ => return,
    };
    if let Some(value) = args.get_mut(slot) {
        *value = canonical_language(value).to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_arguments;

    #[test]
    fn only_language_slot_is_rewritten() {
        let mut args = vec!["lint", "py", "py", "--ruff-tool", "/tmp/py"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        normalize_arguments(&mut args);
        assert_eq!(args, ["lint", "python", "py", "--ruff-tool", "/tmp/py"]);
    }

    #[test]
    fn grammar_dialects_and_unknown_spellings_keep_identity() {
        for raw in [
            vec!["grammar", "probe", "ts"],
            vec!["task", "show", "py"],
            vec!["plan", "check", "tsx"],
            vec!["check", "PY"],
            vec!["lint", "js"],
            vec!["lint", "bash"],
        ] {
            let mut args = raw.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
            normalize_arguments(&mut args);
            assert_eq!(args, raw);
        }
    }

    #[test]
    fn every_alias_targets_an_existing_registry_identity() {
        let registry = codeguard_adapters::legacy_registry().unwrap();
        for alias in ["py", "rs", "ts", "rb", "kt", "erl", "golang", "c++", "c#"] {
            assert!(
                registry
                    .languages
                    .iter()
                    .any(|row| row.id == super::canonical_language(alias))
            );
        }
    }
}
