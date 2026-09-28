//! 原生 Go 包清单的文件选择观察，不能证明所有平台或规则覆盖。
use crate::go_list_package::GoListPackage;
use std::collections::BTreeSet;
use std::path::{Component, Path};
/// 本机默认构建条件下的包文件集合。
#[derive(Debug)]
pub struct GoListScope {
    /// 普通源码及内外部测试源码。
    pub active_files: BTreeSet<String>,
    /// 原生构建约束排除的源码。
    pub ignored_files: BTreeSet<String>,
    /// 完整解析的本地包数。
    pub package_count: usize,
}
/// 解析 go list 的连续 JSON 对象；坏字段、归属或错误使整份清单无效。
/// root 为当前模块绝对根，返回路径相对该根；不执行工具或读取源码。
pub fn parse_go_list_scope(root: &Path, bytes: &[u8]) -> Result<GoListScope, &'static str> {
    if !root.is_absolute() || bytes.len() > 8 * 1024 * 1024 {
        return Err("go_list_report_invalid");
    }
    let mut scope = GoListScope {
        active_files: BTreeSet::new(),
        ignored_files: BTreeSet::new(),
        package_count: 0,
    };
    let mut packages = BTreeSet::new();
    for item in serde_json::Deserializer::from_slice(bytes).into_iter::<GoListPackage>() {
        let package = item.map_err(|_| "go_list_report_invalid")?;
        if package.incomplete
            || package.dependency_only
            || package.go_root
            || package.error.is_some()
            || !package.dependency_errors.is_empty()
            || package.import_path.is_empty()
            || package.name.is_empty()
            || package.import_path.chars().any(char::is_control)
            || package.name.chars().any(char::is_control)
            || !packages.insert((package.import_path, package.directory.clone()))
        {
            return Err("go_list_package_invalid");
        }
        let directory = Path::new(&package.directory);
        if package.directory.chars().any(char::is_control)
            || !directory.is_absolute()
            || directory
                .components()
                .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
        {
            return Err("go_list_path_invalid");
        }
        let relative = directory
            .strip_prefix(root)
            .map_err(|_| "go_list_path_invalid")?;
        for (active, files) in [
            (true, package.go_files),
            (true, package.cgo_files),
            (true, package.test_go_files),
            (true, package.external_test_files),
            (false, package.ignored_files),
        ] {
            for file in files {
                if !file.ends_with(".go")
                    || file.contains(['/', '\\', ':'])
                    || file.chars().any(char::is_control)
                {
                    return Err("go_list_path_invalid");
                }
                let path = relative
                    .join(file)
                    .to_str()
                    .ok_or("go_list_path_invalid")?
                    .replace('\\', "/");
                let set = if active {
                    &mut scope.active_files
                } else {
                    &mut scope.ignored_files
                };
                if !set.insert(path) {
                    return Err("go_list_source_conflict");
                }
                if scope.active_files.len() + scope.ignored_files.len() > 10_000 {
                    return Err("go_list_source_limit");
                }
            }
        }
        scope.package_count += 1;
        if scope.package_count > 1024 {
            return Err("go_list_package_limit");
        }
    }
    if scope.package_count == 0 || scope.active_files.is_empty() {
        return Err("go_list_scope_empty");
    }
    if !scope.active_files.is_disjoint(&scope.ignored_files) {
        return Err("go_list_source_conflict");
    }
    Ok(scope)
}
