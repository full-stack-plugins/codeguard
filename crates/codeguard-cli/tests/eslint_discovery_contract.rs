use codeguard_adapters::legacy_registry;
use codeguard_cli::discovery::discover;
use codeguard_core::{ObservationPort, ObservedPathKind};
use std::{
    cell::Cell,
    io,
    path::{Path, PathBuf},
};
struct ChangingObservation {
    reads: Cell<u32>,
    config: bool,
    changed: bool,
    unreadable: bool,
}
impl ObservationPort for ChangingObservation {
    fn classify(&self, path: &Path) -> io::Result<ObservedPathKind> {
        Ok(if path == Path::new("/fixture") {
            ObservedPathKind::Directory
        } else {
            ObservedPathKind::File
        })
    }
    fn children(&self, _: &Path) -> io::Result<Vec<PathBuf>> {
        let mut paths = vec![PathBuf::from("/fixture/package.json")];
        if self.config {
            paths.push(PathBuf::from("/fixture/eslint.config.js"));
        }
        Ok(paths)
    }
    fn read_bounded(&self, path: &Path, _: u64) -> io::Result<Vec<u8>> {
        if path.file_name().unwrap() == "package.json" {
            let count = self.reads.get() + 1;
            self.reads.set(count);
            if count >= 3 && self.unreadable {
                return Err(io::Error::new(io::ErrorKind::PermissionDenied, "fixture"));
            }
            return Ok(if count >= 3 && self.changed {
                br#"{"version":"2.0.0"}"#.to_vec()
            } else {
                br#"{"version":"1.0.0"}"#.to_vec()
            });
        }
        Ok(b"export default []".to_vec())
    }
}
#[test]
fn changed_or_unreadable_manifest_invalidates_discovery_even_with_flat_config() {
    for config in [false, true] {
        for unreadable in [false, true] {
            let observation = ChangingObservation {
                reads: Cell::new(0),
                config,
                changed: !unreadable,
                unreadable,
            };
            let report = discover(
                Path::new("/fixture"),
                &legacy_registry().unwrap(),
                &observation,
            );
            assert!(!report.observation_complete);
            assert!(
                report
                    .blocked_paths
                    .iter()
                    .any(|path| path == "package.json")
            );
            assert!(
                report
                    .checker_configurations
                    .iter()
                    .any(|entry| entry.checker_id == "node.eslint"
                        && entry.reason == "eslint_manifest_input_unavailable_or_changed")
            );
            assert!(
                report
                    .checker_configurations
                    .iter()
                    .any(|entry| entry.checker_id == "node.npm.audit"
                        && entry.configuration == "unknown"
                        && entry.reason == "npm_manifest_input_unavailable_or_changed")
            );
            assert!(
                !report
                    .checker_configurations
                    .iter()
                    .any(|entry| entry.configuration == "configured")
            );
        }
    }
}
