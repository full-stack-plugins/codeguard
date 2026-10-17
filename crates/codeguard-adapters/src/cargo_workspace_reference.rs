//! Cargo 显式工作区引用的有界路径计划；不访问文件系统或计算成员归属。

/// 一份显式工作区目录引用的项目内候选路径及必须逐项核验的目录。
#[derive(Debug, Eq, PartialEq)]
pub struct CargoWorkspaceReference {
    /// 项目根相对的候选清单；仅在全部遍历目录和当前摘要核验后可读取。
    pub manifest: String,
    /// 按遍历顺序保留目录，包括被父目录组件消除的路径；不能跳过缺失或链接。
    pub traversal_directories: Vec<String>,
}

impl CargoWorkspaceReference {
    /// 观察至多256KiB的成员清单和相对来源，返回显式引用路径计划或未声明。
    /// 非UTF8/TOML、根与成员双角色、非法/绝对/非便携路径、越界和64组件预算返回未解析原因。
    pub fn observe(member: &[u8], source: &str) -> Result<Option<Self>, &'static str> {
        if member.len() > 256 * 1024 {
            return Err("cargo_workspace_reference_manifest_invalid");
        }
        let parsed: toml::Value = std::str::from_utf8(member)
            .ok()
            .and_then(|s| s.parse().ok())
            .ok_or("cargo_workspace_reference_manifest_invalid")?;
        let Some(reference) = parsed.get("package").and_then(|p| p.get("workspace")) else {
            return Ok(None);
        };
        if parsed.get("workspace").is_some() {
            return Err("cargo_workspace_reference_conflicting_root");
        }
        let reference = reference
            .as_str()
            .filter(|s| portable_relative(s))
            .ok_or("cargo_workspace_reference_invalid_path")?;
        if !portable_relative(source) || source.rsplit('/').next() != Some("Cargo.toml") {
            return Err("cargo_workspace_reference_invalid_source");
        }
        let mut segments: Vec<&str> = source.split('/').collect();
        segments.pop();
        if segments.iter().any(|s| matches!(*s, "" | "." | "..")) {
            return Err("cargo_workspace_reference_invalid_source");
        }
        let reference_segments: Vec<&str> = reference.split('/').collect();
        if segments.len() + reference_segments.len() > 64 {
            return Err("cargo_workspace_reference_budget_exceeded");
        }
        let mut directories = vec![".".into()];
        for end in 1..=segments.len() {
            directories.push(segments[..end].join("/"));
        }
        for part in reference_segments {
            match part {
                "" | "." => {}
                ".." => {
                    if segments.pop().is_none() {
                        return Err("cargo_workspace_reference_outside_project");
                    }
                }
                part => segments.push(part),
            }
            directories.push(if segments.is_empty() {
                ".".into()
            } else {
                segments.join("/")
            });
        }
        let prefix = if segments.is_empty() {
            String::new()
        } else {
            format!("{}/", segments.join("/"))
        };
        Ok(Some(Self {
            manifest: format!("{prefix}Cargo.toml"),
            traversal_directories: directories,
        }))
    }
}

fn portable_relative(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 4096
        && !path.starts_with('/')
        && !path.contains(['\\', ':'])
        && !path.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::CargoWorkspaceReference;
    #[test]
    fn explicit_reference_preserves_traversal_and_rejects_escape_or_bad_types() {
        let manifest = |value: &str| format!("[package]\nname='a'\nworkspace={value}\n");
        let bytes = manifest("'./missing/../shared/'");
        let r = CargoWorkspaceReference::observe(bytes.as_bytes(), "member/Cargo.toml")
            .unwrap()
            .unwrap();
        assert_eq!(r.manifest, "member/shared/Cargo.toml");
        assert!(
            r.traversal_directories
                .iter()
                .any(|d| d == "member/missing")
        );
        assert_eq!(
            CargoWorkspaceReference::observe(
                manifest("'../shared'").as_bytes(),
                "member/Cargo.toml"
            )
            .unwrap()
            .unwrap()
            .manifest,
            "shared/Cargo.toml"
        );
        for raw in [
            "'../../outside'",
            "'/absolute'",
            "'C:/absolute'",
            "true",
            "[]",
            "''",
            "'a\\b'",
        ] {
            assert!(
                CargoWorkspaceReference::observe(manifest(raw).as_bytes(), "member/Cargo.toml")
                    .is_err(),
                "{raw}"
            );
        }
        assert!(
            CargoWorkspaceReference::observe(
                b"[package]\nworkspace='.'\n[workspace]\n",
                "Cargo.toml"
            )
            .is_err()
        );
        assert!(
            CargoWorkspaceReference::observe(manifest("'.'").as_bytes(), "../Cargo.toml").is_err()
        );
        assert!(
            CargoWorkspaceReference::observe(
                manifest(&format!("'{}'", "a/".repeat(65))).as_bytes(),
                "member/Cargo.toml"
            )
            .is_err()
        );
        assert!(
            CargoWorkspaceReference::observe(b"[package]\nname='a'\n", "Cargo.toml")
                .unwrap()
                .is_none()
        );
    }
}
