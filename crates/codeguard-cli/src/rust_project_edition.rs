use codeguard_adapters::CargoEditionDeclaration;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

/// Rust选中文件的静态edition上下文；来源：Cargo声明及OpenSpec编辑解析契约。
/// 保留查找途中不存在的清单，避免运行中新增更近清单改变适用edition。
pub struct RustProjectEdition {
    /// 已确定的edition，不等价于本机工具链版本。
    pub edition: &'static str,
    root: PathBuf,
    relative: String,
    package_manifest: PathBuf,
    workspace_manifest: Option<PathBuf>,
    inputs: Vec<(PathBuf, Option<Vec<u8>>)>,
}
impl RustProjectEdition {
    /// 从项目根内的选中文件读取最近包及必要工作区声明；不执行Cargo或源码。
    /// 返回声明上下文；越界、链接、歧义和无清单返回具体未完成原因。
    pub fn capture(root: &Path, relative: &str) -> Result<Self, &'static str> {
        if !root.is_absolute()
            || root.canonicalize().ok().as_deref() != Some(root)
            || relative.is_empty()
            || Path::new(relative)
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err("rust_edition_source_scope_invalid");
        }
        crate::plain_syntax_source::read_plain_source(&root.join(relative))?;
        let source = root.join(relative);
        let mut inputs = Vec::new();
        let mut package = None;
        for parent in source
            .parent()
            .ok_or("rust_edition_source_scope_invalid")?
            .ancestors()
            .take(64)
        {
            if !parent.starts_with(root) {
                break;
            }
            let path = parent.join("Cargo.toml");
            let bytes = read_manifest(&path)?;
            inputs.push((path.clone(), bytes.clone()));
            if let Some(bytes) = bytes {
                package = Some((path, bytes));
                break;
            }
        }
        let (package_manifest, bytes) = package.ok_or("rust_package_manifest_not_found")?;
        let declaration = CargoEditionDeclaration::observe(&bytes)?;
        let mut workspace_manifest = None;
        let edition = if !declaration.inherits_workspace {
            declaration.resolve(None)?
        } else if let Some(locator) = declaration.workspace_locator.as_deref() {
            // 显式相对定位允许..；规范化后必须仍在请求根内，逐祖先读取拒绝链接。
            let candidate = package_manifest
                .parent()
                .unwrap()
                .join(locator)
                .canonicalize()
                .map_err(|_| "rust_workspace_locator_unavailable")?;
            if !candidate.starts_with(root) {
                return Err("rust_workspace_locator_outside_root");
            }
            // 规范化前也拒绝链接目录，避免别名定位被当成确定来源。
            let raw = package_manifest.parent().unwrap().join(locator);
            let mut prefix = PathBuf::new();
            for component in raw.components() {
                prefix.push(component);
                if fs::symlink_metadata(&prefix)
                    .map_err(|_| "rust_workspace_locator_unavailable")?
                    .file_type()
                    .is_symlink()
                {
                    return Err("rust_workspace_locator_symlink");
                }
            }
            let path = candidate.join("Cargo.toml");
            let bytes = read_manifest(&path)?.ok_or("rust_workspace_edition_unresolved")?;
            let edition = declaration.resolve(Some(&bytes))?;
            inputs.push((path.clone(), Some(bytes)));
            workspace_manifest = Some(path);
            edition
        } else {
            let mut resolved = None;
            for parent in package_manifest.parent().unwrap().ancestors().take(64) {
                if !parent.starts_with(root) {
                    break;
                }
                let path = parent.join("Cargo.toml");
                let bytes = read_manifest(&path)?;
                inputs.push((path.clone(), bytes.clone()));
                if let Some(bytes) = bytes {
                    if let Some(edition) = CargoEditionDeclaration::workspace_edition(&bytes)? {
                        workspace_manifest = Some(path);
                        resolved = Some(edition);
                        break;
                    }
                }
            }
            resolved.ok_or("rust_workspace_edition_unresolved")?
        };
        Ok(Self {
            edition,
            root: root.into(),
            relative: relative.into(),
            package_manifest,
            workspace_manifest,
            inputs,
        })
    }

    /// 重新读取相同定位链并比较字节、缺失清单和edition；返回当前上下文是否仍适用。
    pub fn current(&self) -> bool {
        Self::capture(&self.root, &self.relative).is_ok_and(|now| {
            now.inputs == self.inputs
                && now.edition == self.edition
                && now.package_manifest == self.package_manifest
                && now.workspace_manifest == self.workspace_manifest
        })
    }

    /// 返回不含绝对宿主路径的来源摘要；参数为空，输出相对清单与字节身份。
    pub fn report(&self) -> serde_json::Value {
        use sha2::{Digest, Sha256};
        let hash = |path: &Path| {
            self.inputs
                .iter()
                .find(|(p, _)| p == path)
                .and_then(|(_, bytes)| bytes.as_ref())
                .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        };
        serde_json::json!({"edition":self.edition,"manifest_ref":self.package_manifest.strip_prefix(&self.root).ok(),"manifest_sha256":hash(&self.package_manifest),"workspace_manifest_ref":self.workspace_manifest.as_ref().and_then(|p|p.strip_prefix(&self.root).ok()),"workspace_manifest_sha256":self.workspace_manifest.as_ref().and_then(|p|hash(p))})
    }
}
fn read_manifest(path: &Path) -> Result<Option<Vec<u8>>, &'static str> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("rust_edition_manifest_unreadable"),
        Ok(meta) if !meta.file_type().is_file() || meta.len() > 256 * 1024 => {
            Err("rust_edition_manifest_not_regular_or_too_large")
        }
        Ok(_) => crate::plain_syntax_source::read_plain_source(path).map(Some),
    }
}
