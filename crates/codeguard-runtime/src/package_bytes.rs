//! 在展开或发布之前验证有界发行包流。
use ring::digest::{Context, SHA256};
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// 校验传输流的精确长度及 SHA-256，返回冻结包字节供后续展开或发布。
/// 参数为传输读取器、锁定长度/摘要、共享截止时间和取消标记；失败仅返回脱敏原因。
/// 不批准发行来源、不执行/展开/发布；读取器必须自行保证单次 read 的截止时间，
/// 本函数不能中断任意阻塞 Read，只在每次读取前后检查共享预算。
pub fn read_verified_package(
    reader: &mut impl Read,
    expected_size: u64,
    expected_sha256: [u8; 32],
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, &'static str> {
    if !(1..=134217728).contains(&expected_size) || expected_sha256 == [0; 32] {
        return Err("package_input_invalid");
    }
    check_budget(deadline, cancelled)?;
    let mut bytes = Vec::new();
    let mut hash = Context::new(&SHA256);
    let mut chunk = [0; 64 * 1024];
    loop {
        check_budget(deadline, cancelled)?;
        // 超出声明长度至多读取一个字节，不能让恶意无穷流继续占用内存。
        let capacity = ((expected_size - bytes.len() as u64) + 1).min(chunk.len() as u64) as usize;
        let read = match reader.read(&mut chunk[..capacity]) {
            Ok(count) if count <= capacity => count,
            Ok(_) => return Err("package_read_failed"),
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err("package_read_failed"),
        };
        check_budget(deadline, cancelled)?;
        if read == 0 {
            break;
        }
        if bytes.len() as u64 + read as u64 > expected_size {
            return Err("package_size_mismatch");
        }
        bytes
            .try_reserve(read)
            .map_err(|_| "package_allocation_failed")?;
        bytes.extend_from_slice(&chunk[..read]);
        hash.update(&chunk[..read]);
    }
    if bytes.len() as u64 != expected_size {
        return Err("package_size_mismatch");
    }
    if hash.finish().as_ref() != expected_sha256 {
        return Err("package_digest_mismatch");
    }
    check_budget(deadline, cancelled)?;
    Ok(bytes)
}

fn check_budget(deadline: Instant, cancelled: &AtomicBool) -> Result<(), &'static str> {
    if cancelled.load(Ordering::Relaxed) || crate::sigint_cancellation_requested() {
        Err("package_cancelled")
    } else if Instant::now() >= deadline {
        Err("package_deadline_exceeded")
    } else {
        Ok(())
    }
}
