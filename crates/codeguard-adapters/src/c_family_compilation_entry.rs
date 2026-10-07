//! 编译数据库的静态条目；不是可执行计划。

use serde::Deserialize;

/// 保存原始工作目录、文件与参数，不赋予执行或项目覆盖资格。
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CFamilyCompilationEntry {
    /// 编译器运行目录，尚未规范化或核验文件系统身份。
    pub directory: String,
    /// 编译单元原始路径，可能相对工作目录。
    pub file: String,
    /// 原始参数数组，尚未审计插件、响应文件或写文件选项。
    pub arguments: Vec<String>,
    /// 同时存在的命令字符串只保留为数据，禁止作为执行回退。
    pub command: Option<String>,
    /// 原始产物声明，不允许据此创建或覆盖文件。
    pub output: Option<String>,
}

/// 读取最多1MiB、4096条的静态数据库；返回完整原序条目或明确阻塞原因。
/// 不读取路径、不执行参数，不隐式补标准，不合并同文件的不同配置。
pub fn parse_c_family_compilation_database(
    bytes: &[u8],
) -> Result<Vec<CFamilyCompilationEntry>, &'static str> {
    if bytes.len() > 1024 * 1024 {
        return Err("compilation_database_byte_limit");
    }
    let entries: Vec<CFamilyCompilationEntry> =
        serde_json::from_slice(bytes).map_err(|_| "compilation_database_shape_unresolved")?;
    if entries.is_empty() || entries.len() > 4096 {
        return Err("compilation_database_entry_limit");
    }
    for entry in &entries {
        if entry.directory.trim().is_empty() || entry.file.trim().is_empty() {
            return Err("compilation_database_path_unresolved");
        }
        if entry.arguments.is_empty() || entry.arguments.len() > 256 {
            return Err("compilation_database_arguments_unresolved");
        }
        if entry.arguments[0].trim().is_empty()
            || entry
                .arguments
                .iter()
                .any(|arg| arg.contains('\0') || arg.len() > 16 * 1024)
            || entry.directory.contains('\0')
            || entry.file.contains('\0')
        {
            return Err("compilation_database_argument_limit_or_invalid_bytes");
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::parse_c_family_compilation_database;

    #[test]
    fn preserves_conflicting_configurations_and_unexecuted_arguments() {
        let entries = parse_c_family_compilation_database(br#"[
          {"directory":"/build/a", "file":"../src/a.cpp", "arguments":["clang++","-Iinclude one","-DVALUE=1","-std=c++17","-c","../src/a.cpp"]},
          {"directory":"/build/b", "file":"../src/a.cpp", "arguments":["clang++","-Iother","-DVALUE=2","@flags.rsp","$(touch forbidden)","-c","../src/a.cpp"],"command":"never execute this"}
        ]"#).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].arguments[1], "-Iinclude one");
        assert_eq!(entries[1].directory, "/build/b");
        assert_eq!(entries[1].arguments[3], "@flags.rsp");
        assert_eq!(entries[1].arguments[4], "$(touch forbidden)");
        assert!(!entries[1].arguments.iter().any(|a| a.starts_with("-std=")));
    }

    #[test]
    fn malformed_or_command_only_entries_never_become_clean_scope() {
        for bytes in [
            "[]",
            "{}",
            "null",
            r#"[{"directory":"/a","file":"a.cpp","command":"clang++ a.cpp"}]"#,
            r#"[{"directory":"/a","file":"a.cpp","arguments":[]}]"#,
            r#"[{"directory":"/a","file":"a.cpp","arguments":"clang++ a.cpp"}]"#,
            r#"[{"directory":"/a","directory":"/b","file":"a.cpp","arguments":["clang++"]}]"#,
            r#"[{"directory":"","file":"a.cpp","arguments":["clang++"]}]"#,
            r#"[{"directory":"/a","file":"a.cpp","arguments":["clang++","\u0000"]}]"#,
        ] {
            assert!(
                parse_c_family_compilation_database(bytes.as_bytes()).is_err(),
                "{bytes}"
            );
        }
        let row = serde_json::json!({"directory":"/a", "file":"a.cpp", "arguments":["clang++"]});
        let rows = vec![row; 4097];
        assert!(parse_c_family_compilation_database(&serde_json::to_vec(&rows).unwrap()).is_err());
        for arguments in [
            vec!["clang++".to_owned(); 257],
            vec!["clang++".into(), "x".repeat(16 * 1024 + 1)],
        ] {
            let row =
                serde_json::json!([{"directory":"/a", "file":"a.cpp", "arguments":arguments}]);
            assert!(
                parse_c_family_compilation_database(&serde_json::to_vec(&row).unwrap()).is_err()
            );
        }
        assert!(parse_c_family_compilation_database(&vec![b' '; 1024 * 1024 + 1]).is_err());
    }
}
