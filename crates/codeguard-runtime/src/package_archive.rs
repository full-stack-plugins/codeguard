//! 仅向内存展开受限归档，不调用归档库的文件系统解包接口。
use crate::{
    ArchiveUnpackRequest, UnpackedArchive, UnpackedFile, tar_extension_state::TarExtensionState,
};
use flate2::bufread::GzDecoder;
use ring::digest::{SHA256, digest};
use std::collections::BTreeSet;
use std::io::{Cursor, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// 核对包摘要、所有路径/类型/大小及入口摘要，返回冻结普通文件集合。
/// 参数为包字节、展开请求及共享预算；失败不返回局部成功、不写文件或授予安装权限。
/// tar_gz 支持有界 GNU 长路径与局部 PAX，拒绝链接/sparse；ZIP 支持 stored/deflate。
pub fn unpack_package_archive(
    package: &[u8],
    request: &ArchiveUnpackRequest<'_>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<Vec<UnpackedFile>, &'static str> {
    unpack_package_archive_tree(package, request, deadline, cancelled).map(|tree| tree.files)
}
/// 展开完整普通树并保留空目录/父目录；参数和预算与文件入口相同。
/// 返回冻结完整树供 bundle 校验，旧文件集合入口继续可用，不写文件或授予安装。
pub fn unpack_package_archive_tree(
    package: &[u8],
    request: &ArchiveUnpackRequest<'_>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<UnpackedArchive, &'static str> {
    budget(deadline, cancelled)?;
    if package.is_empty()
        || package.len() > 128 * 1024 * 1024
        || !(1..=536870912).contains(&request.max_unpacked_bytes)
        || request.package_sha256 == [0; 32]
        || request.entrypoint_sha256 == [0; 32]
        || !safe_path(request.entrypoint)
    {
        return Err("archive_input_invalid");
    }
    if digest(&SHA256, package).as_ref() != request.package_sha256 {
        return Err("archive_package_digest_mismatch");
    }
    budget(deadline, cancelled)?;
    let mut files = Vec::new();
    let mut names = BTreeSet::new();
    let mut directories = BTreeSet::new();
    let mut total = 0u64;
    match request.format {
        "tar_gz" => {
            // 除文件内容预算外，最多给头、目录和零填充 16 MiB；完整读到 gzip 尾部以核验 CRC。
            let mut decoder = GzDecoder::new(Cursor::new(package));
            let tar_bytes = read_bytes(
                &mut decoder,
                request.max_unpacked_bytes + 16 * 1024 * 1024,
                deadline,
                cancelled,
            )?;
            if decoder.into_inner().position() != package.len() as u64 {
                return Err("archive_trailing_data");
            }
            let mut extensions = TarExtensionState::default();
            let mut member_count = 0;
            let mut last_end = 0usize;
            loop {
                budget(deadline, cancelled)?;
                let block = tar_bytes
                    .get(last_end..last_end + 512)
                    .ok_or("archive_invalid")?;
                if block.iter().all(|byte| *byte == 0) {
                    break;
                }
                member_count += 1;
                if member_count > 10000 {
                    return Err("archive_entry_limit_exceeded");
                }
                let header = tar::Header::from_byte_slice(block);
                // 用归档库的头字段解析核验；扩展 size 决定数据边界，不能沿原头大小前进。
                let checksum = block[..148]
                    .iter()
                    .chain(&block[156..])
                    .map(|byte| *byte as u32)
                    .sum::<u32>()
                    + 8 * 32;
                if header.cksum().map_err(|_| "archive_invalid")? != checksum {
                    return Err("archive_invalid");
                }
                let kind = header.entry_type();
                let header_size = header.entry_size().map_err(|_| "archive_invalid")?;
                let is_extension = kind.is_gnu_longname()
                    || kind.is_pax_local_extensions()
                    || kind.is_pax_global_extensions();
                if !is_extension && !kind.is_file() && !kind.is_dir() {
                    return Err("archive_entry_type_unsupported");
                }
                if !is_extension
                    && !extensions.has_path_override()
                    && (header.as_bytes()[..100].contains(&b'\\')
                        || header.as_ustar().is_some_and(|h| h.prefix.contains(&b'\\')))
                {
                    return Err("archive_path_invalid");
                }
                let raw = header.path_bytes();
                let (name, size) = if is_extension {
                    (String::new(), header_size)
                } else {
                    extensions.consume(&raw, header_size)?
                };
                if is_extension
                    && (size > 64 * 1024
                        || (header.as_gnu().is_none() && header.as_ustar().is_none()))
                {
                    return Err("archive_extension_invalid");
                }
                if !is_extension {
                    let path = if kind.is_dir() {
                        name.strip_suffix('/').unwrap_or(&name)
                    } else {
                        &name
                    };
                    register(path, kind.is_dir(), &mut names)?;
                    if kind.is_dir() {
                        directories.insert(path.to_owned());
                    }
                    if kind.is_dir() && size != 0 {
                        return Err("archive_invalid");
                    }
                    total = total.checked_add(size).ok_or("archive_size_exceeded")?;
                    if total > request.max_unpacked_bytes {
                        return Err("archive_size_exceeded");
                    }
                }
                let data_start = last_end + 512;
                let data_end = (data_start as u64)
                    .checked_add(size)
                    .ok_or("archive_size_exceeded")?;
                let padded_end = (data_start as u64)
                    .checked_add(
                        size.div_ceil(512)
                            .checked_mul(512)
                            .ok_or("archive_size_exceeded")?,
                    )
                    .ok_or("archive_size_exceeded")?;
                if padded_end > tar_bytes.len() as u64 {
                    return Err("archive_invalid");
                }
                let data = &tar_bytes[data_start..data_end as usize];
                last_end = padded_end as usize;
                if is_extension {
                    extensions.observe(kind.as_byte(), data)?;
                    continue;
                }
                if kind.is_file() {
                    files.push(UnpackedFile {
                        relative_path: name,
                        bytes: read_bytes(&mut Cursor::new(data), size, deadline, cancelled)?,
                    });
                }
            }
            if extensions.has_pending() {
                return Err("archive_extension_orphaned");
            }
            // 终止头之后只允许零填充，不忽略第二份 tar 或隐藏数据。
            if last_end.saturating_add(1024) > tar_bytes.len()
                || tar_bytes[last_end..].iter().any(|b| *b != 0)
            {
                return Err("archive_trailing_data");
            }
        }
        "zip" => {
            let mut archive =
                zip::ZipArchive::new(Cursor::new(package)).map_err(|_| "archive_invalid")?;
            // 库允许定位到带附加数据的 EOCD；本协议要求注释后就是包尾，拒绝 SFX 前缀。
            let footer = package
                .len()
                .checked_sub(archive.comment().len() + 22)
                .ok_or("archive_invalid")?;
            if archive.offset() != 0
                || package[footer..footer + 4] != *b"PK\x05\x06"
                || u16::from_le_bytes([package[footer + 20], package[footer + 21]]) as usize
                    != archive.comment().len()
                || &package[footer + 22..] != archive.comment()
            {
                return Err("archive_trailing_data");
            }
            if archive.len() > 10000 {
                return Err("archive_entry_limit_exceeded");
            }
            for index in 0..archive.len() {
                budget(deadline, cancelled)?;
                let mut entry = archive.by_index(index).map_err(|_| "archive_invalid")?;
                let dir = entry.is_dir();
                let mode = entry.unix_mode().unwrap_or(0) & 0o170000;
                if entry.encrypted()
                    || !matches!(
                        entry.compression(),
                        zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
                    )
                    || (mode != 0 && mode != if dir { 0o040000 } else { 0o100000 })
                {
                    return Err("archive_entry_type_unsupported");
                }
                let name = std::str::from_utf8(entry.name_raw())
                    .map_err(|_| "archive_path_invalid")?
                    .to_owned();
                let path = if dir {
                    name.strip_suffix('/').unwrap_or(&name)
                } else {
                    &name
                };
                register(path, dir, &mut names)?;
                if dir {
                    directories.insert(path.to_owned());
                }
                let size = entry.size();
                if dir {
                    if size != 0 {
                        return Err("archive_invalid");
                    }
                    continue;
                }
                total = total.checked_add(size).ok_or("archive_size_exceeded")?;
                if total > request.max_unpacked_bytes {
                    return Err("archive_size_exceeded");
                }
                let bytes = read_bytes(&mut entry, size, deadline, cancelled)?;
                if bytes.len() as u64 != size {
                    return Err("archive_invalid");
                }
                files.push(UnpackedFile {
                    relative_path: path.to_owned(),
                    bytes,
                });
            }
        }
        _ => return Err("archive_format_unsupported"),
    }
    let entry = files
        .iter()
        .find(|file| file.relative_path == request.entrypoint)
        .ok_or("archive_entrypoint_missing")?;
    if digest(&SHA256, &entry.bytes).as_ref() != request.entrypoint_sha256 {
        return Err("archive_entrypoint_digest_mismatch");
    }
    budget(deadline, cancelled)?;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let tree = UnpackedArchive::from_parts(files, directories)?;
    budget(deadline, cancelled)?;
    Ok(tree)
}
/// 核对内存归档树的规范相对路径，不接受平台歧义或回退。
pub(crate) fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 1024
        && !path.contains(['\\', ':', '<', '>', '"', '|', '?', '*'])
        && path.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && !part.ends_with(['.', ' '])
                && !part.chars().any(char::is_control)
                && !reserved_name(part)
        })
}
fn register(
    path: &str,
    dir: bool,
    names: &mut BTreeSet<(String, bool)>,
) -> Result<(), &'static str> {
    if !safe_path(path) {
        return Err("archive_path_invalid");
    }
    if names.len() >= 10000 {
        return Err("archive_entry_limit_exceeded");
    }
    let key = path.to_lowercase();
    // 同名、大小写碰撞、文件/目录及祖先文件冲突均在落盘之前拒绝。
    for (old, old_dir) in names.iter() {
        if old == &key
            || (!old_dir && key.starts_with(&format!("{old}/")))
            || (!dir && old.starts_with(&format!("{key}/")))
        {
            return Err("archive_path_conflict");
        }
    }
    names.insert((key, dir));
    Ok(())
}
fn read_bytes(
    reader: &mut impl Read,
    limit: u64,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, &'static str> {
    let mut result = Vec::new();
    let mut chunk = [0; 64 * 1024];
    loop {
        budget(deadline, cancelled)?;
        let capacity = (limit - result.len() as u64 + 1).min(chunk.len() as u64) as usize;
        let count = reader
            .read(&mut chunk[..capacity])
            .map_err(|_| "archive_read_failed")?;
        budget(deadline, cancelled)?;
        if count == 0 {
            break;
        }
        if count > capacity || result.len() as u64 + count as u64 > limit {
            return Err("archive_size_exceeded");
        }
        result
            .try_reserve(count)
            .map_err(|_| "archive_allocation_failed")?;
        result.extend_from_slice(&chunk[..count]);
    }
    Ok(result)
}
fn budget(deadline: Instant, cancelled: &AtomicBool) -> Result<(), &'static str> {
    if cancelled.load(Ordering::Relaxed) || crate::sigint_cancellation_requested() {
        Err("archive_cancelled")
    } else if Instant::now() >= deadline {
        Err("archive_deadline_exceeded")
    } else {
        Ok(())
    }
}

fn reserved_name(part: &str) -> bool {
    let base = part.split('.').next().unwrap_or("").to_ascii_uppercase();
    matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((base.starts_with("COM") || base.starts_with("LPT"))
            && base.len() == 4
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
}
