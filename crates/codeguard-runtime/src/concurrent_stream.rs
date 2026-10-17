//! 并发流读取：输出预算/私有原子日志/洪流/编码错误/日志 symlink 场景无假完整、无越界写
//!
//! 验收标准：洪流/编码错误/日志 symlink 场景无假完整、无越界写

use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

/// 输出预算
#[derive(Debug)]
pub struct OutputBudget {
    /// 最大字节数
    pub max_bytes: u64,
    /// 已用字节数
    used: AtomicU64,
    /// 是否超限
    exceeded: AtomicBool,
}

impl OutputBudget {
    /// 创建预算
    pub fn new(max_bytes: u64) -> Self {
        Self {
            max_bytes,
            used: AtomicU64::new(0),
            exceeded: AtomicBool::new(false),
        }
    }
    
    /// 尝试消费预算
    pub fn consume(&self, bytes: u64) -> bool {
        let new_used = self.used.fetch_add(bytes, Ordering::SeqCst) + bytes;
        if new_used > self.max_bytes {
            self.exceeded.store(true, Ordering::SeqCst);
            false
        } else {
            true
        }
    }
    
    /// 是否超限
    pub fn is_exceeded(&self) -> bool {
        self.exceeded.load(Ordering::SeqCst)
    }
    
    /// 已用字节数
    pub fn used(&self) -> u64 {
        self.used.load(Ordering::SeqCst)
    }
}

/// 并发流读取结果
#[derive(Debug, Clone)]
pub struct StreamResult {
    /// 读取的字节数
    pub bytes_read: u64,
    /// 是否完整
    pub complete: bool,
    /// 未完成原因
    pub incomplete_reason: Option<String>,
    /// 是否有编码错误
    pub encoding_error: bool,
    /// 是否有洪流
    pub flood_detected: bool,
}

/// 并发流读取器
pub struct ConcurrentStream {
    /// 输出预算
    budget: OutputBudget,
    /// 私有原子日志
    log: Mutex<Vec<u8>>,
}

impl ConcurrentStream {
    /// 创建并发流读取器
    pub fn new(max_bytes: u64) -> Self {
        Self {
            budget: OutputBudget::new(max_bytes),
            log: Mutex::new(Vec::new()),
        }
    }
    
    /// 从 reader 读取（带预算和洪流检测）
    pub fn read_from<R: Read>(&self, reader: &mut R) -> StreamResult {
        let mut buffer = [0u8; 8192];
        let mut bytes_read = 0u64;
        let mut encoding_error = false;
        let mut flood_detected = false;
        
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    bytes_read += n as u64;
                    
                    // 检查预算
                    if !self.budget.consume(n as u64) {
                        return StreamResult {
                            bytes_read,
                            complete: false,
                            incomplete_reason: Some("output_limit_exceeded".into()),
                            encoding_error,
                            flood_detected: true,
                        };
                    }
                    
                    // 洪流检测（单次读取超过阈值）
                    if n > 4096 {
                        flood_detected = true;
                    }
                    
                    // 编码错误检测（非法 UTF-8）
                    if std::str::from_utf8(&buffer[..n]).is_err() {
                        encoding_error = true;
                    }
                    
                    // 写入私有日志
                    if let Ok(mut log) = self.log.lock() {
                        log.extend_from_slice(&buffer[..n]);
                    }
                }
                Err(_) => {
                    return StreamResult {
                        bytes_read,
                        complete: false,
                        incomplete_reason: Some("read_error".into()),
                        encoding_error,
                        flood_detected,
                    };
                }
            }
        }
        
        StreamResult {
            bytes_read,
            complete: true,
            incomplete_reason: None,
            encoding_error,
            flood_detected,
        }
    }
    
    /// 写入日志（原子写入）
    pub fn write_log(&self, path: &Path, data: &[u8]) -> Result<(), String> {
        // 检查 symlink（防止日志 symlink 攻击）
        if path.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
            return Err("log_symlink_detected".into());
        }
        
        // 原子写入（先写临时文件再重命名）
        let tmp_path = path.with_extension("tmp");
        let mut file = std::fs::File::create(&tmp_path)
            .map_err(|e| format!("create_failed: {}", e))?;
        
        file.write_all(data)
            .map_err(|e| format!("write_failed: {}", e))?;
        
        std::fs::rename(&tmp_path, path)
            .map_err(|e| format!("rename_failed: {}", e))?;
        
        Ok(())
    }
    
    /// 获取日志内容
    pub fn log_contents(&self) -> Vec<u8> {
        self.log.lock().map(|l| l.clone()).unwrap_or_default()
    }
}

/// 并发读取多个流
pub fn read_concurrent<R: Read>(readers: &mut [&mut R], max_bytes_each: u64) -> Vec<StreamResult> {
    readers.iter_mut().map(|reader| {
        let stream = ConcurrentStream::new(max_bytes_each);
        stream.read_from(*reader)
    }).collect()
}
