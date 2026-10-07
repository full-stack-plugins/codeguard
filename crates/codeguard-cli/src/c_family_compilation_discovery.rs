//! 有界编译数据库配置观察，不执行项目命令。

use crate::discovery::DiscoveryReport;
use codeguard_adapters::{CheckerConfiguration, parse_c_family_compilation_database};
use codeguard_core::ObservationPort;
use sha2::{Digest, Sha256};
use std::path::Path;

/// 保存配置摘要与未完成原因；参数为同次发现报告、观察端口和数据库路径。
pub(crate) fn observe<P: ObservationPort>(
    report: &mut DiscoveryReport,
    observation: &P,
    path: &Path,
    relative: &str,
) {
    let mut reason = "compilation_database_unreadable";
    let mut parsed = false;
    if let Ok(bytes) = observation.read_bounded(path, 1024 * 1024) {
        if observation
            .read_bounded(path, 1024 * 1024)
            .is_ok_and(|current| current == bytes)
        {
            report
                .checker_config_sha256
                .insert(relative.into(), format!("{:x}", Sha256::digest(&bytes)));
            reason = match parse_c_family_compilation_database(&bytes) {
                Ok(entries) => {
                    parsed = true;
                    entries
                        .iter()
                        .find_map(|entry| entry.execution_context_blocker())
                        .unwrap_or(
                            "compilation_database_arguments_observed_execution_context_unverified",
                        )
                }
                Err(reason) => reason,
            };
        } else {
            reason = "compilation_database_changed";
        }
    }
    if !parsed {
        report.observation_complete = false;
        report.blocked_paths.push(relative.into());
    }
    report
        .unknown_conditions
        .push(format!("{reason}:{relative}"));
    let build_root = relative.rsplit_once('/').map_or(".", |(parent, _)| parent);
    report.checker_configurations.push(CheckerConfiguration {
        build_root: build_root.into(),
        checker_id: "c_family.compilation_database".into(),
        category: "lint".into(),
        configuration: "unknown".into(),
        configuration_ref: relative.into(),
        reason: reason.into(),
        next_action: "核验原编译数据库的工具、目录、参数、响应文件和输入身份；不要执行command字符串，不猜标准，不以配置可读证明项目检查通过".into(),
    });
}

#[cfg(test)]
mod tests {
    use codeguard_core::{ObservationPort, ObservedPathKind};
    use std::io;
    use std::path::{Path, PathBuf};
    use std::cell::Cell;

    struct DatabaseObservation {
        bytes: &'static [u8],
        changed: bool,
        reads: Cell<u32>,
    }
    impl ObservationPort for DatabaseObservation {
        fn classify(&self, _: &Path) -> io::Result<ObservedPathKind> {
            Ok(ObservedPathKind::File)
        }
        fn children(&self, _: &Path) -> io::Result<Vec<PathBuf>> {
            Ok(vec![])
        }
        fn read_bounded(&self, _: &Path, limit: u64) -> io::Result<Vec<u8>> {
            assert_eq!(limit, 1024 * 1024);
            self.reads.set(self.reads.get() + 1);
            if self.changed && self.reads.get() > 1 {
                return Ok(b"[]".to_vec());
            }
            Ok(self.bytes.to_vec())
        }
    }

    #[test]
    fn discovered_database_keeps_unverified_config_and_bound_digest() {
        let mut report = crate::discovery::empty_report(Path::new("project"));
        let observation = DatabaseObservation { bytes: br#"[{"directory":"/build","file":"a.cpp","arguments":["clang++","-Iinclude","a.cpp"]}]"#, changed:false, reads:Cell::new(0) };
        crate::discovery::observe_file(
            &mut report,
            &codeguard_adapters::legacy_registry().unwrap(),
            &observation,
            Path::new("project/build/compile_commands.json"),
            "build/compile_commands.json",
            &std::collections::BTreeSet::new(),
        );
        assert!(
            report
                .checker_config_sha256
                .contains_key("build/compile_commands.json")
        );
        assert_eq!(report.checker_configurations[0].configuration, "unknown");
        assert_eq!(report.checker_configurations[0].build_root, "build");
        assert!(report.declared_versions.is_empty());
        assert!(report.build_roots.is_empty());
    }

    #[test]
    fn unstable_or_command_only_database_never_becomes_verified_config() {
        for (bytes, changed) in [
            (
                br#"[{"directory":"/build","file":"a.cpp","command":"clang++ a.cpp"}]"#.as_slice(),
                false,
            ),
            (
                br#"[{"directory":"/build","file":"a.cpp","arguments":["clang++","a.cpp"]}]"#
                    .as_slice(),
                true,
            ),
        ] {
            let mut report = crate::discovery::empty_report(Path::new("project"));
            let observation = DatabaseObservation {
                bytes,
                changed,
                reads: Cell::new(0),
            };
            super::observe(
                &mut report,
                &observation,
                Path::new("project/compile_commands.json"),
                "compile_commands.json",
            );
            assert!(!report.observation_complete);
            assert_eq!(report.blocked_paths, ["compile_commands.json"]);
            assert_eq!(report.checker_configurations[0].configuration, "unknown");
            if changed {
                assert!(report.checker_config_sha256.is_empty());
            }
        }
    }
}
