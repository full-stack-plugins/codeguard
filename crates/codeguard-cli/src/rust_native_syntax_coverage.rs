//! 仅在同一进程内传递当前 Clippy 编译目标的源码身份，不授予项目覆盖或门禁权威。

#[cfg(feature = "wasm-precheck")]
use crate::rust_lint_inputs::RustLintInputs;
#[cfg(feature = "wasm-precheck")]
use codeguard_adapters::parse_unique_json;
#[cfg(feature = "wasm-precheck")]
use serde_json::Value;
#[cfg(feature = "wasm-precheck")]
use sha2::{Digest, Sha256};
#[cfg(feature = "wasm-precheck")]
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// 本轮完整原生机器流明确编译的目标入口；字节固定于原生启动前。
/// 不从 reports/ 中恢复，也不把 all-targets 推导成所有模块已检查。
#[derive(Default)]
pub(crate) struct RustNativeSyntaxCoverage {
    #[cfg(feature = "wasm-precheck")]
    source_hashes: BTreeMap<String, String>,
}

impl RustNativeSyntaxCoverage {
    /// 从成功且输入稳定的 Clippy 输出提取非缓存目标入口。
    /// 参数为原根、已捕获输入、观察范围和原生机器流；任何流歧义返回空覆盖。
    #[cfg(feature = "wasm-precheck")]
    pub(crate) fn from_completed_stream(
        root: &Path,
        inputs: &RustLintInputs,
        source_files: &BTreeSet<String>,
        bytes: &[u8],
    ) -> Self {
        let Ok(stream) = std::str::from_utf8(bytes) else {
            return Self::default();
        };
        let mut source_hashes = BTreeMap::new();
        let mut finished = false;
        let mut artifact_count = 0;
        for line in stream.lines().filter(|line| !line.trim().is_empty()) {
            if finished {
                return Self::default();
            }
            let Ok(event) = parse_unique_json(line.as_bytes()) else {
                return Self::default();
            };
            match event["reason"].as_str() {
                Some("build-finished") if event["success"] == true => finished = true,
                Some("compiler-artifact") => {
                    artifact_count += 1;
                    if artifact_count > 10_000 {
                        return Self::default();
                    }
                    if event["fresh"] != false
                        || event["manifest_path"].as_str() != root.join("Cargo.toml").to_str()
                        || !event["package_id"]
                            .as_str()
                            .is_some_and(|id| !id.is_empty())
                        || !supported_kind(&event["target"]["kind"])
                    {
                        continue;
                    }
                    let Some(relative) = event["target"]["src_path"]
                        .as_str()
                        .and_then(|source| Path::new(source).strip_prefix(root).ok())
                        .and_then(Path::to_str)
                        .filter(|source| source_files.contains(*source))
                    else {
                        continue;
                    };
                    let Some(source) = inputs.source(relative) else {
                        continue;
                    };
                    source_hashes
                        .insert(relative.to_owned(), format!("{:x}", Sha256::digest(source)));
                }
                Some("compiler-message" | "build-script-executed" | "text-line") => {}
                _ => return Self::default(),
            }
        }
        if finished {
            Self { source_hashes }
        } else {
            Self::default()
        }
    }

    /// 核对候选阶段当前字节与原生启动前字节；参数为工作区相对路径及源码。
    /// 返回 true 只表示该目标入口免重复解析，不证明全部 features/模块覆盖。
    #[cfg(feature = "wasm-precheck")]
    pub(crate) fn covers(&self, relative: &str, source: &[u8]) -> bool {
        self.source_hashes
            .get(relative)
            .is_some_and(|digest| *digest == format!("{:x}", Sha256::digest(source)))
    }
}

#[cfg(feature = "wasm-precheck")]
fn supported_kind(value: &Value) -> bool {
    value.as_array().is_some_and(|kinds| {
        !kinds.is_empty()
            && kinds.len() <= 8
            && kinds.iter().all(|kind| {
                matches!(
                    kind.as_str(),
                    Some(
                        "lib"
                            | "rlib"
                            | "dylib"
                            | "cdylib"
                            | "staticlib"
                            | "proc-macro"
                            | "bin"
                            | "example"
                            | "test"
                            | "bench"
                    )
                )
            })
    })
}

#[cfg(all(test, feature = "wasm-precheck"))]
mod tests {
    use super::RustNativeSyntaxCoverage;
    use crate::rust_lint_inputs::RustLintInputs;
    use serde_json::json;
    use std::{collections::BTreeSet, fs};

    #[test]
    fn coverage_binds_pre_run_bytes_instead_of_source_path_alone() {
        let root = std::env::temp_dir().canonicalize().unwrap().join(format!(
            "codeguard-rust-coverage-hash-{}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='sample'\nversion='0.1.0'\n",
        )
        .unwrap();
        let before = b"pub fn answer() -> i32 { 42 }\n";
        fs::write(root.join("src/lib.rs"), before).unwrap();
        let sources = BTreeSet::from(["src/lib.rs".to_owned()]);
        let inputs = RustLintInputs::capture(&root, &sources).unwrap();
        let artifact = json!({"reason":"compiler-artifact","package_id":"sample","manifest_path":root.join("Cargo.toml"),"target":{"kind":["lib"],"src_path":root.join("src/lib.rs")},"fresh":false});
        let stream = format!("{artifact}\n{{\"reason\":\"build-finished\",\"success\":true}}\n");
        let coverage = RustNativeSyntaxCoverage::from_completed_stream(
            &root,
            &inputs,
            &sources,
            stream.as_bytes(),
        );
        assert!(coverage.covers("src/lib.rs", before));
        fs::write(root.join("src/lib.rs"), "pub fn broken( {\n").unwrap();
        assert!(!coverage.covers("src/lib.rs", &fs::read(root.join("src/lib.rs")).unwrap()));
        assert!(!coverage.covers("src/other.rs", before));
        fs::remove_dir_all(root).unwrap();
    }
}
