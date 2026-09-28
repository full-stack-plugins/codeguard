use super::{authority, configured_client, download_verified_package, download_with_client};
use crate::PackageDownloadRequest;
use crate::download_test_server::DownloadTestServer;
use ring::digest::{SHA256, digest};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
fn request(url: &str) -> PackageDownloadRequest<'_> {
    PackageDownloadRequest {
        url,
        expected_size: 4,
        expected_sha256: digest(&SHA256, b"tool").as_ref().try_into().unwrap(),
        redirect_authorities: &[],
    }
}
fn client(deadline: Instant) -> reqwest::Client {
    configured_client(deadline)
        .unwrap()
        .add_root_certificate(
            reqwest::Certificate::from_der(include_bytes!(
                "../tests/fixtures/download_tls/root.der"
            ))
            .unwrap(),
        )
        .build()
        .unwrap()
}
fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(3)
}
#[tokio::test]
async fn https_returns_only_exact_frozen_bytes() {
    let server = DownloadTestServer::new(
        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntool".to_vec(),
        Duration::ZERO,
    );
    let d = deadline();
    assert_eq!(
        download_with_client(&request(&server.url), d, &AtomicBool::new(false), client(d))
            .await
            .unwrap(),
        b"tool"
    );
    assert!(server.seen.load(Ordering::Relaxed));
}
#[tokio::test]
async fn invalid_url_input_and_pre_cancelled_budget_make_no_requests() {
    for url in [
        "http://example.org/tool",
        "https://user:secret@example.org/tool",
        "https://example.org/tool?secret=1",
        "https://example.org/tool#part",
        "https://example.org/\nheader",
    ] {
        assert!(
            download_verified_package(&request(url), deadline(), &AtomicBool::new(false))
                .await
                .is_err()
        );
    }
    assert!(
        download_verified_package(
            &request("https://example.org/tool"),
            Instant::now(),
            &AtomicBool::new(false)
        )
        .await
        .is_err()
    );
    assert!(
        download_verified_package(
            &request("https://example.org/tool"),
            deadline(),
            &AtomicBool::new(true)
        )
        .await
        .is_err()
    );
}
#[tokio::test]
async fn bad_status_size_digest_encoding_or_short_stream_never_returns_bytes() {
    for (response, expected) in [
        (
            b"HTTP/1.1 404 Nope\r\nContent-Length: 4\r\n\r\ntool".as_slice(),
            "package_download_http_status",
        ),
        (
            b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\ntool!",
            "package_size_mismatch",
        ),
        (
            b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nevil",
            "package_digest_mismatch",
        ),
        (
            b"HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Length: 4\r\n\r\ntool",
            "package_download_content_encoding_invalid",
        ),
        (
            b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nto",
            "package_download_read_failed",
        ),
        (
            b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\ntool!",
            "package_size_mismatch",
        ),
    ] {
        let server = DownloadTestServer::new(response.to_vec(), Duration::ZERO);
        let d = deadline();
        assert_eq!(
            download_with_client(&request(&server.url), d, &AtomicBool::new(false), client(d))
                .await
                .unwrap_err(),
            expected
        );
        assert!(server.seen.load(Ordering::Relaxed));
    }
}
#[tokio::test]
async fn untrusted_tls_certificate_is_rejected_by_production_client() {
    let server = DownloadTestServer::new(
        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\ntool".to_vec(),
        Duration::ZERO,
    );
    assert!(
        download_verified_package(&request(&server.url), deadline(), &AtomicBool::new(false))
            .await
            .is_err()
    );
    assert!(!server.seen.load(Ordering::Relaxed));
}
#[tokio::test]
async fn blocked_network_wait_observes_deadline_and_cancellation() {
    for cancel in [false, true] {
        let server = DownloadTestServer::new(
            b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\ntool".to_vec(),
            Duration::from_secs(2),
        );
        let flag = AtomicBool::new(false);
        let start = Instant::now();
        let d = if cancel {
            deadline()
        } else {
            start + Duration::from_millis(200)
        };
        let c = client(d);
        let signal = async {
            let end = deadline();
            while !server.seen.load(Ordering::Relaxed) && Instant::now() < end {
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
            if server.seen.load(Ordering::Relaxed) {
                flag.store(true, Ordering::Relaxed);
            }
        };
        let req = request(&server.url);
        let result = if cancel {
            tokio::join!(download_with_client(&req, d, &flag, c), signal).0
        } else {
            download_with_client(&request(&server.url), d, &flag, c).await
        };
        assert_eq!(
            result.unwrap_err(),
            if cancel {
                "package_download_cancelled"
            } else {
                "package_download_deadline_exceeded"
            }
        );
        assert!(start.elapsed() < Duration::from_secs(1));
    }
}
#[tokio::test]
async fn cross_origin_redirect_needs_explicit_authority_and_keeps_shared_budget() {
    let target = DownloadTestServer::new(
        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\ntool".to_vec(),
        Duration::ZERO,
    );
    let source = DownloadTestServer::new(
        format!(
            "HTTP/1.1 302 Found\r\nLocation: {}\r\nContent-Length: 0\r\n\r\n",
            target.url
        )
        .into_bytes(),
        Duration::ZERO,
    );
    let d = deadline();
    assert_eq!(
        download_with_client(&request(&source.url), d, &AtomicBool::new(false), client(d))
            .await
            .unwrap_err(),
        "package_download_redirect_unapproved"
    );
    assert!(!target.seen.load(Ordering::Relaxed));
    let source = DownloadTestServer::new(
        format!(
            "HTTP/1.1 302 Found\r\nLocation: {}\r\nContent-Length: 0\r\n\r\n",
            target.url
        )
        .into_bytes(),
        Duration::ZERO,
    );
    let authority = authority(&reqwest::Url::parse(&target.url).unwrap()).unwrap();
    let allowed = [authority.as_str()];
    let req = PackageDownloadRequest {
        redirect_authorities: &allowed,
        ..request(&source.url)
    };
    let d = deadline();
    assert_eq!(
        download_with_client(&req, d, &AtomicBool::new(false), client(d))
            .await
            .unwrap(),
        b"tool"
    );
}

#[tokio::test]
async fn chunked_and_close_delimited_bodies_preserve_exact_package_bytes() {
    for response in [
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nto\r\n2\r\nol\r\n0\r\n\r\n"
            .as_slice(),
        b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\ntool",
    ] {
        let server = DownloadTestServer::new(response.to_vec(), Duration::ZERO);
        let d = deadline();
        assert_eq!(
            download_with_client(&request(&server.url), d, &AtomicBool::new(false), client(d))
                .await
                .unwrap(),
            b"tool"
        );
        assert!(server.seen.load(Ordering::Relaxed));
    }
}
#[tokio::test]
async fn redirect_downgrade_credentials_and_ambiguous_locations_are_rejected() {
    for (headers, expected) in [
        (
            "Location: http://127.0.0.1/package\r\n",
            "package_download_url_invalid",
        ),
        (
            "Location: https://user:secret@127.0.0.1/package\r\n",
            "package_download_url_invalid",
        ),
        ("", "package_download_redirect_invalid"),
        (
            "Location: /one\r\nLocation: /two\r\n",
            "package_download_redirect_invalid",
        ),
    ] {
        let server = DownloadTestServer::new(
            format!("HTTP/1.1 302 Found\r\n{headers}Content-Length: 0\r\n\r\n").into_bytes(),
            Duration::ZERO,
        );
        let d = deadline();
        assert_eq!(
            download_with_client(&request(&server.url), d, &AtomicBool::new(false), client(d))
                .await
                .unwrap_err(),
            expected
        );
        assert!(server.seen.load(Ordering::Relaxed));
    }
}

#[tokio::test]
async fn transfer_encoding_must_be_only_chunked_without_content_length() {
    let server = DownloadTestServer::new(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip, chunked\r\n\r\n4\r\ntool\r\n0\r\n\r\n"
            .to_vec(),
        Duration::ZERO,
    );
    let d = deadline();
    assert_eq!(
        download_with_client(&request(&server.url), d, &AtomicBool::new(false), client(d))
            .await
            .unwrap_err(),
        "package_download_framing_invalid"
    );
    assert!(server.seen.load(Ordering::Relaxed));
}

#[tokio::test]
async fn redirect_chain_stops_before_a_sixth_hop() {
    let mut servers = vec![DownloadTestServer::new(
        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\ntool".to_vec(),
        Duration::ZERO,
    )];
    for _ in 0..6 {
        let next = &servers.last().unwrap().url;
        servers.push(DownloadTestServer::new(
            format!("HTTP/1.1 302 Found\r\nLocation: {next}\r\nContent-Length: 0\r\n\r\n")
                .into_bytes(),
            Duration::ZERO,
        ));
    }
    let authorities: Vec<_> = servers
        .iter()
        .map(|s| authority(&reqwest::Url::parse(&s.url).unwrap()).unwrap())
        .collect();
    let allowed: Vec<_> = authorities.iter().map(String::as_str).collect();
    let req = PackageDownloadRequest {
        redirect_authorities: &allowed,
        ..request(&servers.last().unwrap().url)
    };
    let d = deadline();
    assert_eq!(
        download_with_client(&req, d, &AtomicBool::new(false), client(d))
            .await
            .unwrap_err(),
        "package_download_redirect_limit"
    );
    assert!(!servers[0].seen.load(Ordering::Relaxed));
    assert!(servers[1..].iter().all(|s| s.seen.load(Ordering::Relaxed)));
}
#[tokio::test]
async fn redirect_target_cannot_reset_original_deadline() {
    let target = DownloadTestServer::new(
        b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\ntool".to_vec(),
        Duration::from_secs(2),
    );
    let source = DownloadTestServer::new(
        format!(
            "HTTP/1.1 302 Found\r\nLocation: {}?signed=test-only\r\nContent-Length: 0\r\n\r\n",
            target.url
        )
        .into_bytes(),
        Duration::from_millis(80),
    );
    let authority = authority(&reqwest::Url::parse(&target.url).unwrap()).unwrap();
    let allowed = [authority.as_str()];
    let req = PackageDownloadRequest {
        redirect_authorities: &allowed,
        ..request(&source.url)
    };
    let start = Instant::now();
    let d = start + Duration::from_millis(350);
    let error = download_with_client(&req, d, &AtomicBool::new(false), client(d))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        "package_download_deadline_exceeded" | "package_download_timeout"
    ));
    assert!(source.seen.load(Ordering::Relaxed));
    assert!(target.seen.load(Ordering::Relaxed));
    assert!(start.elapsed() < Duration::from_secs(1));
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[tokio::test]
async fn downloaded_archive_verifies_full_tree_before_private_publication() {
    use std::io::{Cursor, Write};
    use std::os::unix::fs::PermissionsExt;
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .add_directory("empty/", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer
        .start_file("bin/tool", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"tool").unwrap();
    let zip_package = writer.finish().unwrap().into_inner();
    let mut tar = tar::Builder::new(Vec::new());
    for (path, bytes, kind) in [
        ("empty/", b"".as_slice(), tar::EntryType::Directory),
        ("bin/tool", b"tool".as_slice(), tar::EntryType::Regular),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o755);
        header.set_entry_type(kind);
        header.set_cksum();
        tar.append_data(&mut header, path, bytes).unwrap();
    }
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(&tar.into_inner().unwrap()).unwrap();
    let gzip_package = gz.finish().unwrap();
    for (format, package) in [("zip", zip_package), ("tar_gz", gzip_package)] {
        let sha: [u8; 32] = digest(&SHA256, &package).as_ref().try_into().unwrap();
        let archive = crate::ArchiveUnpackRequest {
            format,
            entrypoint: "bin/tool",
            package_sha256: sha,
            entrypoint_sha256: request("unused").expected_sha256,
            max_unpacked_bytes: 1024,
        };
        let flag = AtomicBool::new(false);
        let expected_tree =
            crate::unpack_package_archive_tree(&package, &archive, deadline(), &flag).unwrap();
        let expected = crate::hash_unpacked_bundle_tree(&expected_tree, deadline(), &flag).unwrap();
        for invalid_tree in [false, true] {
            let mut response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                package.len()
            )
            .into_bytes();
            response.extend_from_slice(&package);
            let server = DownloadTestServer::new(response, Duration::ZERO);
            let req = PackageDownloadRequest {
                url: &server.url,
                expected_size: package.len() as u64,
                expected_sha256: sha,
                redirect_authorities: &[],
            };
            let d = deadline();
            let declared = if invalid_tree {
                "f".repeat(64)
            } else {
                expected.clone()
            };
            let result =
                super::download_archive_with_client(&req, &archive, &declared, d, &flag, client(d))
                    .await;
            assert!(server.seen.load(Ordering::Relaxed));
            if invalid_tree {
                assert_eq!(result.unwrap_err(), "bundle_digest_mismatch");
            } else {
                let tree = result.unwrap();
                assert!(tree.directories.iter().any(|v| v == "empty"));
                let root = std::env::temp_dir().join(format!(
                    "codeguard-https-archive-{}-{}",
                    std::process::id(),
                    server
                        .url
                        .rsplit(':')
                        .next()
                        .unwrap()
                        .split('/')
                        .next()
                        .unwrap()
                ));
                std::fs::create_dir(&root).unwrap();
                std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
                let published =
                    crate::publish_tool_bundle(&root, &tree, &expected, d, &flag).unwrap();
                assert_eq!(
                    std::fs::read(root.join(&published.relative_path).join("bin/tool")).unwrap(),
                    b"tool"
                );
                assert!(root.join(&published.relative_path).join("empty").is_dir());
                std::fs::remove_dir_all(root).unwrap();
            }
        }
    }
}
#[tokio::test]
async fn conflicting_archive_declarations_are_rejected_before_network() {
    let server = DownloadTestServer::new(Vec::new(), Duration::ZERO);
    let req = request(&server.url);
    let archive = crate::ArchiveUnpackRequest {
        format: "zip",
        entrypoint: "bin/tool",
        package_sha256: [1; 32],
        entrypoint_sha256: req.expected_sha256,
        max_unpacked_bytes: 1024,
    };
    let d = deadline();
    assert_eq!(
        crate::download_verified_archive(
            &req,
            &archive,
            &"a".repeat(64),
            d,
            &AtomicBool::new(false)
        )
        .await
        .unwrap_err(),
        "package_archive_declaration_mismatch"
    );
    assert!(!server.seen.load(Ordering::Relaxed));
}

#[cfg(unix)]
#[tokio::test]
async fn downloaded_raw_bytes_publish_only_after_transport_identity_matches() {
    use std::os::unix::fs::PermissionsExt;
    for corrupt in [false, true] {
        let body = if corrupt { "evil" } else { "tool" };
        let server = DownloadTestServer::new(
            format!("HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\n{body}").into_bytes(),
            Duration::ZERO,
        );
        let req = request(&server.url);
        let d = deadline();
        let flag = AtomicBool::new(false);
        let result = download_with_client(&req, d, &flag, client(d)).await;
        assert!(server.seen.load(Ordering::Relaxed));
        if corrupt {
            assert_eq!(result.unwrap_err(), "package_digest_mismatch");
        } else {
            let bytes = result.unwrap();
            let root = std::env::temp_dir().join(format!(
                "codeguard-https-raw-{}-{}",
                std::process::id(),
                server
                    .url
                    .rsplit(':')
                    .next()
                    .unwrap()
                    .split('/')
                    .next()
                    .unwrap()
            ));
            std::fs::create_dir(&root).unwrap();
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
            let installed =
                crate::publish_tool_bytes(&root, &bytes, req.expected_sha256, d, &flag).unwrap();
            assert_eq!(
                std::fs::read(root.join(installed.relative_path)).unwrap(),
                b"tool"
            );
            std::fs::remove_dir_all(root).unwrap();
        }
    }
}
#[tokio::test]
async fn invalid_archive_inputs_never_start_a_request() {
    let server = DownloadTestServer::new(Vec::new(), Duration::ZERO);
    let req = request(&server.url);
    for (format, path, limit, entry, tree) in [
        ("rar", "bin/tool", 1024, req.expected_sha256, "a".repeat(64)),
        ("zip", "../tool", 1024, req.expected_sha256, "a".repeat(64)),
        ("zip", "bin/tool", 0, req.expected_sha256, "a".repeat(64)),
        ("zip", "bin/tool", 1024, [0; 32], "a".repeat(64)),
        ("zip", "bin/tool", 1024, req.expected_sha256, "0".repeat(64)),
        ("zip", "bin/tool", 1024, req.expected_sha256, "A".repeat(64)),
    ] {
        let archive = crate::ArchiveUnpackRequest {
            format,
            entrypoint: path,
            package_sha256: req.expected_sha256,
            entrypoint_sha256: entry,
            max_unpacked_bytes: limit,
        };
        assert_eq!(
            crate::download_verified_archive(
                &req,
                &archive,
                &tree,
                deadline(),
                &AtomicBool::new(false)
            )
            .await
            .unwrap_err(),
            "package_archive_input_invalid"
        );
    }
    assert!(!server.seen.load(Ordering::Relaxed));
}
