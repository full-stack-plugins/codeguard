//! 并发流读取测试

use codeguard_runtime::concurrent_stream::*;
use std::io::Cursor;

#[test]
fn budget_consume_within_limit() {
    let budget = OutputBudget::new(100);
    assert!(budget.consume(50));
    assert!(budget.consume(50));
    assert_eq!(budget.used(), 100);
    assert!(!budget.is_exceeded());
}

#[test]
fn budget_consume_exceeds_limit() {
    let budget = OutputBudget::new(100);
    assert!(budget.consume(60));
    assert!(!budget.consume(50));
    assert!(budget.is_exceeded());
}

#[test]
fn stream_read_within_budget() {
    let stream = ConcurrentStream::new(1024);
    let data = b"hello world";
    let mut cursor = Cursor::new(data);
    
    let result = stream.read_from(&mut cursor);
    assert!(result.complete);
    assert_eq!(result.bytes_read, 11);
    assert!(!result.encoding_error);
    assert!(!result.flood_detected);
}

#[test]
fn stream_read_exceeds_budget() {
    let stream = ConcurrentStream::new(5);
    let data = b"this is a long string";
    let mut cursor = Cursor::new(data);
    
    let result = stream.read_from(&mut cursor);
    assert!(!result.complete);
    assert_eq!(result.incomplete_reason, Some("output_limit_exceeded".into()));
    assert!(result.flood_detected);
}

#[test]
fn stream_detects_encoding_error() {
    let stream = ConcurrentStream::new(1024);
    let data = vec![0xFF, 0xFE, 0xFD]; // 非法 UTF-8
    let mut cursor = Cursor::new(data);
    
    let result = stream.read_from(&mut cursor);
    assert!(result.encoding_error);
}

#[test]
fn stream_detects_flood() {
    let stream = ConcurrentStream::new(1024 * 1024);
    let data = vec![0u8; 8192]; // 超过 4096 阈值
    let mut cursor = Cursor::new(data);
    
    let result = stream.read_from(&mut cursor);
    assert!(result.flood_detected);
}

#[test]
fn stream_log_symlink_rejected() {
    let tmp = std::env::temp_dir().join("test_symlink_log");
    let _ = std::fs::remove_file(&tmp);
    
    // 创建 symlink
    #[cfg(unix)]
    std::os::unix::fs::symlink("/etc/passwd", &tmp).unwrap();
    
    let stream = ConcurrentStream::new(1024);
    let result = stream.write_log(&tmp, b"test");
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "log_symlink_detected");
    
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn stream_log_atomic_write() {
    let tmp = std::env::temp_dir().join("test_atomic_log.txt");
    let _ = std::fs::remove_file(&tmp);
    
    let stream = ConcurrentStream::new(1024);
    let result = stream.write_log(&tmp, b"atomic write test");
    
    assert!(result.is_ok());
    assert!(tmp.exists());
    
    let content = std::fs::read(&tmp).unwrap();
    assert_eq!(content, b"atomic write test");
    
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn stream_no_fake_complete_on_error() {
    let stream = ConcurrentStream::new(1024);
    struct FailingReader;
    impl std::io::Read for FailingReader {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "read error"))
        }
    }
    
    let mut reader = FailingReader;
    let result = stream.read_from(&mut reader);
    
    assert!(!result.complete);
    assert_eq!(result.incomplete_reason, Some("read_error".into()));
}

#[test]
fn stream_no_out_of_bounds_write() {
    let stream = ConcurrentStream::new(1024);
    let data = b"test data";
    let mut cursor = Cursor::new(data);
    
    let result = stream.read_from(&mut cursor);
    let log = stream.log_contents();
    
    // 日志只包含读取的数据，无越界写
    assert_eq!(log.len() as u64, result.bytes_read);
}
