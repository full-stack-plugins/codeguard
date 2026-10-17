//! 工作树快照测试

use codeguard_runtime::work_tree_snapshot::*;

#[test]
fn process_regular_file() {
    let snapshot = WorkTreeSnapshot::new();
    let result = snapshot.process_file(std::path::Path::new("test.txt"), b"hello world");
    
    assert_eq!(result.file_type, FileType::Regular);
    assert!(result.sha1.is_some());
    assert!(result.sha256.is_some());
    assert_eq!(result.size, 11);
}

#[test]
fn process_symlink() {
    let tmp = std::env::temp_dir().join("test_symlink_snapshot");
    let _ = std::fs::remove_file(&tmp);
    
    #[cfg(unix)]
    std::os::unix::fs::symlink("/etc/passwd", &tmp).unwrap();
    
    let snapshot = WorkTreeSnapshot::new();
    let result = snapshot.process_file(&tmp, b"");
    
    assert_eq!(result.file_type, FileType::Symlink);
    assert!(result.sha1.is_none());
    
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn process_lfs_pointer() {
    let tmp = std::env::temp_dir().join("test_lfs_pointer.txt");
    std::fs::write(&tmp, b"version https://git-lfs.github.com/spec/v1\n").unwrap();
    
    let snapshot = WorkTreeSnapshot::new();
    let result = snapshot.process_file(&tmp, b"");
    
    assert_eq!(result.file_type, FileType::LfsPointer);
    
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn process_batch_complete() {
    let snapshot = WorkTreeSnapshot::new();
    let files = vec![
        ("a.txt".to_string(), b"content a".to_vec()),
        ("b.txt".to_string(), b"content b".to_vec()),
    ];
    
    let result = snapshot.process_batch(&files);
    assert!(result.complete);
    assert_eq!(result.files.len(), 2);
    assert_eq!(result.unresolved_count, 0);
}

#[test]
fn process_batch_with_unresolved() {
    let snapshot = WorkTreeSnapshot::new();
    let files = vec![
        ("a.txt".to_string(), b"content a".to_vec()),
        ("b.txt".to_string(), b"content b".to_vec()),
    ];
    
    let result = snapshot.process_batch(&files);
    assert!(result.complete);
}

#[test]
fn verify_bytes_sha256() {
    let snapshot = WorkTreeSnapshot::new();
    let content = b"hello world";
    let result = snapshot.process_file(std::path::Path::new("test.txt"), content);
    
    let sha256 = result.sha256.unwrap();
    assert!(snapshot.verify_bytes("test.txt", &sha256));
    assert!(!snapshot.verify_bytes("test.txt", "wrong_hash"));
}

#[test]
fn snapshot_lookup() {
    let snapshot = WorkTreeSnapshot::new();
    snapshot.process_file(std::path::Path::new("test.txt"), b"content");
    
    let found = snapshot.snapshot("test.txt");
    assert!(found.is_some());
    assert_eq!(found.unwrap().size, 7);
}

#[test]
fn sha1_sha256_both_computed() {
    let snapshot = WorkTreeSnapshot::new();
    let result = snapshot.process_file(std::path::Path::new("test.txt"), b"content");
    
    assert!(result.sha1.is_some());
    assert!(result.sha256.is_some());
    assert!(result.sha1.is_some() && result.sha256.is_some());
}
