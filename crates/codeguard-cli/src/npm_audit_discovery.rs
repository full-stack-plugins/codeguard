//! npm审计声明的逐构建根只读发现；不执行package脚本、不授权漏洞覆盖。
use crate::discovery::DiscoveryReport;
use codeguard_adapters::inspect_npm_audit_config;
use codeguard_core::ObservationPort;
use sha2::{Digest, Sha256};
use std::path::Path;

pub(crate) fn inspect_node_npm_audit<P: ObservationPort>(
    root: &Path,
    observation: &P,
    report: &mut DiscoveryReport,
) {
    let manifests: Vec<_> = report
        .manifest_sha256
        .iter()
        .filter(|(path, _)| {
            Path::new(path)
                .file_name()
                .is_some_and(|name| name == "package.json")
        })
        .map(|(path, digest)| (path.clone(), digest.clone()))
        .collect();
    for (manifest, expected) in manifests {
        let parent = Path::new(&manifest)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");
        let build_root = if parent.is_empty() { "." } else { parent };
        let config = match observation.read_bounded(&root.join(&manifest), 256 * 1024) {
            Ok(bytes) if format!("{:x}", Sha256::digest(&bytes)) == expected => {
                inspect_npm_audit_config(&bytes, build_root, &manifest)
            }
            _ => {
                report.observation_complete = false;
                report.blocked_paths.push(manifest.clone());
                let mut entry = inspect_npm_audit_config(&[], build_root, &manifest);
                entry.reason = "npm_manifest_input_unavailable_or_changed".into();
                entry.next_action =
                    "恢复可读且稳定的项目清单后重新发现审计配置；不可使用旧脚本声明推断当前已配置"
                        .into();
                entry
            }
        };
        report.checker_configurations.push(config);
    }
}
