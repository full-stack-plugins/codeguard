#[path = "guard_integration_freshness.rs"]
mod fixture;
use codeguard_cli::guard_integration::consumer::consume;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};
fn snapshot(root: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, map: &mut BTreeMap<String, String>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(Result::unwrap)
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let path = e.path();
            let meta = std::fs::symlink_metadata(&path).unwrap();
            assert!(!meta.file_type().is_symlink());
            if meta.is_dir() {
                walk(root, &path, map)
            } else {
                map.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    format!("{:x}", Sha256::digest(std::fs::read(path).unwrap())),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}
#[test]
#[ignore = "explicit existing Git fixture root and strace capture required"]
fn repeated_consumption_leaves_actual_source_events_refs_and_index_unchanged() {
    let root = std::path::PathBuf::from(
        std::env::var_os("CODEGUARD_SIDE_EFFECT_ROOT").expect("explicit fixture root"),
    );
    let before = snapshot(&root);
    assert!(before.contains_key(".git/index"));
    assert!(before.keys().any(|p| p.starts_with(".git/refs/")));
    assert!(before.contains_key(".codeguard/events.jsonl"));
    assert!(before.contains_key("app.py"));
    let (output, expected) = fixture::sample(false);
    let provider = fixture::FixtureProvider::new();
    let report = output.report_bytes().unwrap().to_vec();
    for _ in 0..10 {
        assert!(
            consume(&output, &expected, &provider, fixture::NOW, None)
                .unwrap()
                .eligibility
                .eligible
        );
    }
    assert_eq!(before, snapshot(&root));
    assert_eq!(report, output.report_bytes().unwrap());
}
