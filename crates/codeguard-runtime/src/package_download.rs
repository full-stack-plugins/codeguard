//! HTTPS 发行包传输；只返回已核对字节，不授予来源或安装权威。
use crate::{ArchiveUnpackRequest, PackageDownloadRequest, UnpackedArchive};
use futures_util::TryStreamExt;
use reqwest::{Client, ClientBuilder, Url};
use ring::digest::{Context, SHA256};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
/// 下载同一冻结声明的精确 HTTPS 包，继承共享期限与取消。
/// 参数为 URL/大小/摘要/显式重定向范围及预算；返回有界字节或固定诊断。
/// 不采用环境代理/自动解压、不写文件；调用方须先独立核验发行来源与网络授权。
pub async fn download_verified_package(
    request: &PackageDownloadRequest<'_>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, &'static str> {
    validate(request)?;
    budget(deadline, cancelled)?;
    let client = configured_client(deadline)?
        .build()
        .map_err(|_| "package_download_client_failed")?;
    download_with_client(request, deadline, cancelled, client).await
}
fn configured_client(deadline: Instant) -> Result<ClientBuilder, &'static str> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .filter(|d| !d.is_zero())
        .ok_or("package_download_deadline_exceeded")?;
    Ok(Client::builder()
        .no_proxy()
        .https_only(true)
        .http1_only()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .timeout(remaining)
        .connect_timeout(remaining.min(Duration::from_secs(10)))
        .read_timeout(remaining.min(Duration::from_secs(10)))
        .pool_max_idle_per_host(0)
        .user_agent("codeguard-package-runtime/0.1"))
}
/// 下载并核验完整归档树，保持下载与展开使用同一包声明和预算。
/// 参数包含冻结下载/归档声明、完整树摘要、绝对期限及取消信号。
/// 返回未发布的内存树；调用方须先独立核验来源和网络许可，本函数不安装或授权。
pub async fn download_verified_archive(
    request: &PackageDownloadRequest<'_>,
    archive: &ArchiveUnpackRequest<'_>,
    expected_tree_sha256: &str,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<UnpackedArchive, &'static str> {
    validate_archive(request, archive, expected_tree_sha256)?;
    budget(deadline, cancelled)?;
    let client = configured_client(deadline)?
        .build()
        .map_err(|_| "package_download_client_failed")?;
    download_archive_with_client(
        request,
        archive,
        expected_tree_sha256,
        deadline,
        cancelled,
        client,
    )
    .await
}
fn validate_archive(
    request: &PackageDownloadRequest<'_>,
    archive: &ArchiveUnpackRequest<'_>,
    expected_tree_sha256: &str,
) -> Result<(), &'static str> {
    validate(request)?;
    if request.expected_sha256 != archive.package_sha256 {
        return Err("package_archive_declaration_mismatch");
    }
    if !matches!(archive.format, "zip" | "tar_gz")
        || !crate::package_archive::safe_path(archive.entrypoint)
        || archive.entrypoint_sha256 == [0; 32]
        || !(1..=536870912).contains(&archive.max_unpacked_bytes)
        || expected_tree_sha256.len() != 64
        || !expected_tree_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || expected_tree_sha256.bytes().all(|b| b == b'0')
    {
        return Err("package_archive_input_invalid");
    }
    Ok(())
}
async fn download_archive_with_client(
    request: &PackageDownloadRequest<'_>,
    archive: &ArchiveUnpackRequest<'_>,
    expected_tree_sha256: &str,
    deadline: Instant,
    cancelled: &AtomicBool,
    client: Client,
) -> Result<UnpackedArchive, &'static str> {
    validate_archive(request, archive, expected_tree_sha256)?;
    let bytes = download_with_client(request, deadline, cancelled, client).await?;
    // 包、入口与整树分别核验；下载成功不能绕过归档路径或成员检查。
    let tree = crate::unpack_package_archive_tree(&bytes, archive, deadline, cancelled)?;
    crate::verify_unpacked_bundle_tree(&tree, expected_tree_sha256, deadline, cancelled)?;
    Ok(tree)
}
fn validate(request: &PackageDownloadRequest<'_>) -> Result<(), &'static str> {
    if !(1..=134217728).contains(&request.expected_size)
        || request.expected_sha256 == [0; 32]
        || request.redirect_authorities.len() > 32
    {
        return Err("package_download_input_invalid");
    }
    let url = parse_url(request.url)?;
    if url.query().is_some() {
        return Err("package_download_url_invalid");
    }
    for value in request.redirect_authorities {
        if value.contains('*') {
            return Err("package_download_redirect_scope_invalid");
        }
        let parsed = parse_url(&format!("https://{value}/"))?;
        if parsed.path() != "/"
            || parsed.query().is_some()
            || authority(&parsed)?.as_str() != *value
        {
            return Err("package_download_redirect_scope_invalid");
        }
    }
    Ok(())
}
/// 静态校验冻结下载请求，不建客户端或发出网络请求。
/// 参数为声明与重定向范围；返回固定诊断，不授予网络授权。
pub fn validate_package_download_request(
    request: &PackageDownloadRequest<'_>,
) -> Result<(), &'static str> {
    validate(request)
}
/// 返回合法初始 HTTPS URL 的规范主机与端口，用于宿主精确许可核对。
/// 参数不得含凭据/查询/片段；返回不含原路径的 authority 或固定诊断。
pub fn package_download_authority(value: &str) -> Result<String, &'static str> {
    let url = parse_url(value)?;
    if url.query().is_some() {
        return Err("package_download_url_invalid");
    }
    authority(&url)
}
fn parse_url(value: &str) -> Result<Url, &'static str> {
    if value.len() > 2048 || value.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("package_download_url_invalid");
    }
    let url = Url::parse(value).map_err(|_| "package_download_url_invalid")?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("package_download_url_invalid");
    }
    Ok(url)
}
fn authority(url: &Url) -> Result<String, &'static str> {
    Ok(format!(
        "{}:{}",
        url.host_str().ok_or("package_download_url_invalid")?,
        url.port_or_known_default()
            .ok_or("package_download_url_invalid")?
    ))
}
async fn download_with_client(
    request: &PackageDownloadRequest<'_>,
    deadline: Instant,
    cancelled: &AtomicBool,
    client: Client,
) -> Result<Vec<u8>, &'static str> {
    validate(request)?;
    budget(deadline, cancelled)?;
    let mut url = parse_url(request.url)?;
    let initial_authority = authority(&url)?;
    let response = {
        let mut hops = 0;
        loop {
            budget(deadline, cancelled)?;
            let response = controlled(
                async {
                    client
                        .get(url.clone())
                        .header(reqwest::header::ACCEPT_ENCODING, "identity")
                        .send()
                        .await
                        .map_err(|e| {
                            if e.is_timeout() {
                                "package_download_timeout"
                            } else {
                                "package_download_request_failed"
                            }
                        })
                },
                deadline,
                cancelled,
            )
            .await?;
            if matches!(response.status().as_u16(), 301 | 302 | 303 | 307 | 308) {
                if hops >= 5 {
                    return Err("package_download_redirect_limit");
                }
                let locations: Vec<_> = response
                    .headers()
                    .get_all(reqwest::header::LOCATION)
                    .iter()
                    .collect();
                if locations.len() != 1 {
                    return Err("package_download_redirect_invalid");
                }
                let location = locations[0]
                    .to_str()
                    .map_err(|_| "package_download_redirect_invalid")?;
                if location
                    .chars()
                    .any(|c| c.is_control() || c.is_whitespace())
                {
                    return Err("package_download_redirect_invalid");
                }
                let target = url
                    .join(location)
                    .map_err(|_| "package_download_redirect_invalid")?;
                let target = parse_url(target.as_str())?;
                let target_authority = authority(&target)?;
                if target_authority != initial_authority
                    && !request
                        .redirect_authorities
                        .contains(&target_authority.as_str())
                {
                    return Err("package_download_redirect_unapproved");
                }
                url = target;
                hops += 1;
                continue;
            }
            if response.status() != reqwest::StatusCode::OK {
                return Err("package_download_http_status");
            }
            break response;
        }
    };
    let headers = response.headers();
    let encodings: Vec<_> = headers
        .get_all(reqwest::header::CONTENT_ENCODING)
        .iter()
        .collect();
    if encodings.len() > 1
        || encodings.first().is_some_and(|v| {
            v.to_str()
                .map_or(true, |v| !v.eq_ignore_ascii_case("identity"))
        })
    {
        return Err("package_download_content_encoding_invalid");
    }
    let lengths: Vec<_> = headers
        .get_all(reqwest::header::CONTENT_LENGTH)
        .iter()
        .collect();
    let transfers: Vec<_> = headers
        .get_all(reqwest::header::TRANSFER_ENCODING)
        .iter()
        .collect();
    // 只允许 HTTP 分块框架，不接受其它传输编码隐式改变包字节语义。
    if transfers.len() > 1
        || transfers.first().is_some_and(|value| {
            value
                .to_str()
                .map_or(true, |value| !value.eq_ignore_ascii_case("chunked"))
        })
    {
        return Err("package_download_framing_invalid");
    }
    if lengths.len() > 1
        || (!lengths.is_empty() && headers.contains_key(reqwest::header::TRANSFER_ENCODING))
    {
        return Err("package_download_framing_invalid");
    }
    if let Some(length) = lengths.first() {
        let length = length
            .to_str()
            .ok()
            .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or("package_download_framing_invalid")?;
        if length != request.expected_size {
            return Err("package_size_mismatch");
        }
    }
    let stream = response.bytes_stream().map_err(|e| {
        std::io::Error::new(
            if e.is_timeout() {
                std::io::ErrorKind::TimedOut
            } else {
                std::io::ErrorKind::Other
            },
            "package_transport_read_failed",
        )
    });
    let mut reader = tokio_util::io::StreamReader::new(stream);
    let mut bytes = Vec::new();
    let mut hash = Context::new(&SHA256);
    let mut chunk = [0; 64 * 1024];
    loop {
        budget(deadline, cancelled)?;
        let capacity =
            ((request.expected_size - bytes.len() as u64) + 1).min(chunk.len() as u64) as usize;
        let read = controlled(
            async {
                reader.read(&mut chunk[..capacity]).await.map_err(|e| {
                    if e.kind() == std::io::ErrorKind::TimedOut {
                        "package_download_timeout"
                    } else {
                        "package_download_read_failed"
                    }
                })
            },
            deadline,
            cancelled,
        )
        .await?;
        if read == 0 {
            break;
        }
        if bytes.len() as u64 + read as u64 > request.expected_size {
            return Err("package_size_mismatch");
        }
        bytes
            .try_reserve(read)
            .map_err(|_| "package_allocation_failed")?;
        bytes.extend_from_slice(&chunk[..read]);
        hash.update(&chunk[..read]);
    }
    if bytes.len() as u64 != request.expected_size {
        return Err("package_size_mismatch");
    }
    if hash.finish().as_ref() != request.expected_sha256 {
        return Err("package_digest_mismatch");
    }
    budget(deadline, cancelled)?;
    Ok(bytes)
}
async fn controlled<T>(
    future: impl Future<Output = Result<T, &'static str>>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<T, &'static str> {
    tokio::pin!(future);
    loop {
        budget(deadline, cancelled)?;
        tokio::select! {
            result=&mut future=>{budget(deadline,cancelled)?;return result;}
            _=tokio::time::sleep_until(tokio::time::Instant::from_std(deadline))=>{return Err("package_download_deadline_exceeded");}
            _=tokio::time::sleep(Duration::from_millis(50))=>{}
        }
    }
}
fn budget(deadline: Instant, cancelled: &AtomicBool) -> Result<(), &'static str> {
    if cancelled.load(Ordering::Relaxed) || crate::sigint_cancellation_requested() {
        Err("package_download_cancelled")
    } else if Instant::now() >= deadline {
        Err("package_download_deadline_exceeded")
    } else {
        Ok(())
    }
}
#[cfg(test)]
#[path = "package_download_tests.rs"]
mod tests;
