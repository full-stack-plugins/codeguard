use codeguard_runtime::{ArchiveUnpackRequest, unpack_package_archive};
use flate2::{Compression, write::GzEncoder};
use ring::digest::{SHA256, digest};
use std::io::{Cursor, Write};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};
fn hash(b: &[u8]) -> [u8; 32] {
    digest(&SHA256, b).as_ref().try_into().unwrap()
}
fn unpack(
    format: &str,
    bytes: &[u8],
    entry: &str,
    limit: u64,
) -> Result<Vec<codeguard_runtime::UnpackedFile>, &'static str> {
    unpack_package_archive(
        bytes,
        &ArchiveUnpackRequest {
            format,
            entrypoint: entry,
            package_sha256: hash(bytes),
            entrypoint_sha256: hash(b"tool"),
            max_unpacked_bytes: limit,
        },
        Instant::now() + Duration::from_secs(5),
        &AtomicBool::new(false),
    )
}
fn tar_gz(entries: &[(&str, &[u8], u8)]) -> Vec<u8> {
    let mut tar = tar::Builder::new(Vec::new());
    for (path, bytes, kind) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o755);
        header.set_entry_type(tar::EntryType::new(*kind));
        // 直接写入路径，允许构造路径穿越反例，不让 fixture builder 先过滤。
        let data = header.as_mut_bytes();
        data[..100].fill(0);
        data[..path.len()].copy_from_slice(path.as_bytes());
        header.set_cksum();
        tar.append(&header, *bytes).unwrap();
    }
    let data = tar.into_inner().unwrap();
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&data).unwrap();
    gz.finish().unwrap()
}
fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer.set_comment("fixture-comment");
    for (path, bytes) in entries {
        writer
            .start_file(
                *path,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
#[test]
fn tar_and_zip_produce_verified_in_memory_files() {
    for (format, bytes) in [
        (
            "tar_gz",
            tar_gz(&[("bin/tool", b"tool", b'0'), ("LICENSE", b"license", b'0')]),
        ),
        (
            "zip",
            zip(&[("bin/tool", b"tool"), ("LICENSE", b"license")]),
        ),
    ] {
        let files = unpack(format, &bytes, "bin/tool", 16384).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(
            files
                .iter()
                .find(|f| f.relative_path == "bin/tool")
                .unwrap()
                .bytes,
            b"tool"
        );
    }
}
#[test]
fn archive_paths_and_duplicates_are_rejected() {
    for path in ["../tool", "/tool", "C:\\tool", "bin/../tool", "bin//tool"] {
        for (format, bytes) in [
            ("tar_gz", tar_gz(&[(path, b"tool", b'0')])),
            ("zip", zip(&[(path, b"tool")])),
        ] {
            assert!(unpack(format, &bytes, path, 16384).is_err());
        }
    }
    let bytes = tar_gz(&[("bin/tool", b"tool", b'0'), ("bin/tool", b"tool", b'0')]);
    assert!(unpack("tar_gz", &bytes, "bin/tool", 16384).is_err());
}
#[test]
fn links_devices_missing_entry_bad_digest_and_limits_are_rejected() {
    for kind in *b"12346" {
        assert!(
            unpack(
                "tar_gz",
                &tar_gz(&[("bin/tool", b"", kind)]),
                "bin/tool",
                16384
            )
            .is_err()
        );
    }
    let bytes = zip(&[("bin/other", b"tool")]);
    assert!(unpack("zip", &bytes, "bin/tool", 16384).is_err());
    let bytes = zip(&[("bin/tool", b"evil")]);
    assert!(unpack("zip", &bytes, "bin/tool", 16384).is_err());
    let bytes = zip(&[("bin/tool", b"tool")]);
    assert!(unpack("zip", &bytes, "bin/tool", 3).is_err());
    assert!(unpack("tar_gz", b"broken", "bin/tool", 16384).is_err());
}

#[test]
fn unsafe_secondary_entries_are_not_hidden_by_a_valid_entrypoint() {
    for path in [
        "../other",
        "bin\\other",
        "bin/CON.exe",
        "bin/name.",
        "bin/name ",
        "bin/other?",
        "bin/./other",
    ] {
        for (format, bytes) in [
            (
                "tar_gz",
                tar_gz(&[("bin/tool", b"tool", b'0'), (path, b"bad", b'0')]),
            ),
            ("zip", zip(&[("bin/tool", b"tool"), (path, b"bad")])),
        ] {
            assert!(
                unpack(format, &bytes, "bin/tool", 16384).is_err(),
                "{format}: {path}"
            );
        }
    }
}
#[test]
fn case_collisions_and_file_directory_conflicts_are_rejected() {
    for rows in [
        vec![
            ("bin/tool", b"tool".as_slice()),
            ("BIN/TOOL", b"tool".as_slice()),
        ],
        vec![
            ("bin/tool", b"tool".as_slice()),
            ("bin", b"file".as_slice()),
        ],
    ] {
        assert!(unpack("zip", &zip(&rows), "bin/tool", 16384).is_err());
    }
    let bytes = tar_gz(&[("bin/", b"", b'5'), ("bin/tool", b"tool", b'0')]);
    assert_eq!(
        unpack("tar_gz", &bytes, "bin/tool", 16384).unwrap().len(),
        1
    );
}
#[test]
fn damaged_gzip_trailer_and_extra_members_are_rejected() {
    let original = tar_gz(&[("bin/tool", b"tool", b'0')]);
    let mut damaged = original.clone();
    let n = damaged.len();
    damaged[n - 8] ^= 1;
    assert!(unpack("tar_gz", &damaged, "bin/tool", 16384).is_err());
    let mut extra = original.clone();
    extra.extend_from_slice(&original);
    assert_eq!(
        unpack("tar_gz", &extra, "bin/tool", 16384).unwrap_err(),
        "archive_trailing_data"
    );
    assert!(unpack("tar_gz", &original, "bin/tool", 3).is_err());
    assert!(
        unpack(
            "tar_gz",
            &tar_gz(&[("bin/tool", b"tool", b'0'), ("pax", b"", b'x')]),
            "bin/tool",
            16384
        )
        .is_err()
    );
}
#[test]
fn cancelled_expired_and_wrong_package_digest_never_return_partial_files() {
    let bytes = zip(&[("bin/tool", b"tool")]);
    let mut request = ArchiveUnpackRequest {
        format: "zip",
        entrypoint: "bin/tool",
        package_sha256: hash(&bytes),
        entrypoint_sha256: hash(b"tool"),
        max_unpacked_bytes: 16384,
    };
    assert_eq!(
        unpack_package_archive(
            &bytes,
            &request,
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(true)
        )
        .unwrap_err(),
        "archive_cancelled"
    );
    assert_eq!(
        unpack_package_archive(&bytes, &request, Instant::now(), &AtomicBool::new(false))
            .unwrap_err(),
        "archive_deadline_exceeded"
    );
    request.package_sha256 = hash(b"different");
    assert_eq!(
        unpack_package_archive(
            &bytes,
            &request,
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false)
        )
        .unwrap_err(),
        "archive_package_digest_mismatch"
    );
}

#[test]
fn damaged_zip_data_is_not_accepted_even_when_package_digest_matches() {
    let mut bytes = zip(&[("bin/tool", b"tool")]);
    let offset = {
        let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        archive.by_index(0).unwrap().data_start() as usize
    };
    bytes[offset] ^= 0x80;
    assert!(unpack("zip", &bytes, "bin/tool", 16384).is_err());
}
#[cfg(unix)]
#[test]
fn verified_archive_entry_publishes_through_existing_cache_boundary() {
    use codeguard_runtime::{publish_tool_bytes, read_verified_package};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("cg-archive-chain-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let cancel = AtomicBool::new(false);
    let deadline = Instant::now() + Duration::from_secs(5);
    for (format, package) in [
        ("tar_gz", tar_gz(&[("bin/tool", b"tool", b'0')])),
        ("zip", zip(&[("bin/tool", b"tool")])),
    ] {
        let frozen = read_verified_package(
            &mut Cursor::new(&package),
            package.len() as u64,
            hash(&package),
            deadline,
            &cancel,
        )
        .unwrap();
        let files = unpack_package_archive(
            &frozen,
            &ArchiveUnpackRequest {
                format,
                entrypoint: "bin/tool",
                package_sha256: hash(&frozen),
                entrypoint_sha256: hash(b"tool"),
                max_unpacked_bytes: 16384,
            },
            deadline,
            &cancel,
        )
        .unwrap();
        let file = files
            .iter()
            .find(|f| f.relative_path == "bin/tool")
            .unwrap();
        let receipt =
            publish_tool_bytes(&root, &file.bytes, hash(b"tool"), deadline, &cancel).unwrap();
        assert_eq!(fs::read(root.join(receipt.relative_path)).unwrap(), b"tool");
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn nonzero_data_hidden_after_tar_terminators_is_rejected() {
    let package = tar_gz(&[("bin/tool", b"tool", b'0')]);
    let mut raw = Vec::new();
    std::io::Read::read_to_end(
        &mut flate2::read::GzDecoder::new(Cursor::new(package)),
        &mut raw,
    )
    .unwrap();
    let last = raw.len() - 1;
    raw[last] = b'x';
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw).unwrap();
    let modified = encoder.finish().unwrap();
    assert_eq!(
        unpack("tar_gz", &modified, "bin/tool", 16384).unwrap_err(),
        "archive_trailing_data"
    );
}

#[test]
fn zip_trailing_data_and_self_extracting_prefix_are_rejected() {
    let bytes = zip(&[("bin/tool", b"tool")]);
    let mut suffix = bytes.clone();
    suffix.extend_from_slice(b"hidden");
    assert!(unpack("zip", &suffix, "bin/tool", 16384).is_err());
    let mut prefix = b"self-extractor".to_vec();
    prefix.extend_from_slice(&bytes);
    assert!(unpack("zip", &prefix, "bin/tool", 16384).is_err());
}

fn pax_record(key: &str, value: &str) -> Vec<u8> {
    let payload = format!(" {key}={value}\n");
    let mut length = payload.len() + 1;
    loop {
        let next = payload.len() + length.to_string().len();
        if next == length {
            return format!("{length}{payload}").into_bytes();
        }
        length = next;
    }
}
#[test]
fn gnu_longname_and_local_pax_paths_are_applied_to_the_next_file_only() {
    let long = format!("bin/{}/tool", "segment".repeat(20));
    let mut gnu = long.as_bytes().to_vec();
    gnu.push(0);
    let bytes = tar_gz(&[
        ("././@LongLink", &gnu, b'L'),
        ("placeholder", b"tool", b'0'),
        ("LICENSE", b"license", b'0'),
    ]);
    let files = unpack("tar_gz", &bytes, &long, 16384).unwrap();
    assert!(files.iter().any(|f| f.relative_path == "LICENSE"));
    assert!(files.iter().any(|f| f.relative_path == long));
    let mut pax = pax_record("path", &long);
    pax.extend_from_slice(&pax_record("size", "4"));
    let bytes = tar_gz(&[
        ("PaxHeader", &pax, b'x'),
        ("placeholder", b"tool", b'0'),
        ("LICENSE", b"license", b'0'),
    ]);
    assert_eq!(unpack("tar_gz", &bytes, &long, 16384).unwrap().len(), 2);
}
#[test]
fn harmless_global_pax_metadata_does_not_become_a_file() {
    let data = pax_record("comment", "release fixture");
    let bytes = tar_gz(&[("GlobalPax", &data, b'g'), ("bin/tool", b"tool", b'0')]);
    assert_eq!(
        unpack("tar_gz", &bytes, "bin/tool", 16384).unwrap().len(),
        1
    );
}

#[test]
fn unsafe_duplicate_or_orphaned_extension_paths_are_rejected() {
    for path in ["../tool", "/tool", "bin\\tool", "bin/CON.exe"] {
        let mut gnu = path.as_bytes().to_vec();
        gnu.push(0);
        let pax = pax_record("path", path);
        for bytes in [
            tar_gz(&[("LongLink", &gnu, b'L'), ("placeholder", b"tool", b'0')]),
            tar_gz(&[("PaxHeader", &pax, b'x'), ("placeholder", b"tool", b'0')]),
        ] {
            assert_eq!(
                unpack("tar_gz", &bytes, "bin/tool", 16384).unwrap_err(),
                "archive_path_invalid"
            );
        }
    }
    let mut duplicate = pax_record("path", "bin/tool");
    duplicate.extend_from_slice(&pax_record("path", "bin/other"));
    assert!(
        unpack(
            "tar_gz",
            &tar_gz(&[
                ("PaxHeader", &duplicate, b'x'),
                ("placeholder", b"tool", b'0')
            ]),
            "bin/tool",
            16384
        )
        .is_err()
    );
    assert_eq!(
        unpack(
            "tar_gz",
            &tar_gz(&[("LongLink", b"bin/tool\0", b'L')]),
            "bin/tool",
            16384
        )
        .unwrap_err(),
        "archive_extension_orphaned"
    );
    let bytes = tar_gz(&[
        ("LongLink", b"bin/tool\0", b'L'),
        ("PaxHeader", &pax_record("path", "bin/other"), b'x'),
        ("placeholder", b"tool", b'0'),
    ]);
    assert_eq!(
        unpack("tar_gz", &bytes, "bin/tool", 16384).unwrap_err(),
        "archive_extension_conflict"
    );
}
#[test]
fn malformed_oversized_and_boundary_changing_extensions_are_rejected() {
    for payload in [
        b"99 path=bin/tool\n".to_vec(),
        b"18 path=bin/tool\nTRAILING".to_vec(),
        vec![b'x'; 65537],
        pax_record("size", "18446744073709551616"),
        pax_record("size", "5"),
    ] {
        assert!(
            unpack(
                "tar_gz",
                &tar_gz(&[("PaxHeader", &payload, b'x'), ("bin/tool", b"tool", b'0')]),
                "bin/tool",
                16384
            )
            .is_err()
        );
    }
    for key in ["path", "size", "GNU.sparse.map", "linkpath"] {
        let pax = pax_record(key, if key == "size" { "4" } else { "bin/tool" });
        assert!(
            unpack(
                "tar_gz",
                &tar_gz(&[("GlobalPax", &pax, b'g'), ("bin/tool", b"tool", b'0')]),
                "bin/tool",
                16384
            )
            .is_err()
        );
    }
    for long in [
        b"bin/tool".to_vec(),
        b"bin/tool\0\0".to_vec(),
        vec![b'x'; 1026],
    ] {
        assert!(
            unpack(
                "tar_gz",
                &tar_gz(&[("LongLink", &long, b'L'), ("bin/tool", b"tool", b'0')]),
                "bin/tool",
                16384
            )
            .is_err()
        );
    }
}

#[test]
fn pax_size_overrides_header_and_preserves_the_following_member_boundary() {
    let pax = pax_record("size", "4");
    let package = tar_gz(&[
        ("PaxHeader", &pax, b'x'),
        ("bin/tool", b"tool", b'0'),
        ("LICENSE", b"license", b'0'),
    ]);
    let mut raw = Vec::new();
    std::io::Read::read_to_end(
        &mut flate2::read::GzDecoder::new(Cursor::new(package)),
        &mut raw,
    )
    .unwrap();
    let mut header = tar::Header::new_gnu();
    header.set_path("bin/tool").unwrap();
    header.set_mode(0o755);
    header.set_size(0);
    header.set_cksum();
    raw[1024..1536].copy_from_slice(header.as_bytes());
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw).unwrap();
    let package = encoder.finish().unwrap();
    // 与归档库的标准解释交叉核对：下一条 LICENSE 仍须位于正确块边界。
    let mut reference = tar::Archive::new(Cursor::new(&raw));
    let rows = reference
        .entries()
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.path_bytes().into_owned(), entry.size())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        vec![(b"bin/tool".to_vec(), 4), (b"LICENSE".to_vec(), 7)]
    );
    assert_eq!(
        unpack("tar_gz", &package, "bin/tool", 16384).unwrap().len(),
        2
    );
}

#[test]
fn full_gnu_utf8_path_survives_a_truncated_non_utf8_legacy_header() {
    let path = format!("bin/t{}/tool", "汉".repeat(50));
    let mut long = path.as_bytes().to_vec();
    long.push(0);
    let package = tar_gz(&[("LongLink", &long, b'L'), ("placeholder", b"tool", b'0')]);
    let mut raw = Vec::new();
    std::io::Read::read_to_end(
        &mut flate2::read::GzDecoder::new(Cursor::new(package)),
        &mut raw,
    )
    .unwrap();
    let mut header = tar::Header::new_gnu();
    header.as_mut_bytes().copy_from_slice(&raw[1024..1536]);
    header.as_mut_bytes()[..100].copy_from_slice(&path.as_bytes()[..100]);
    header.set_cksum();
    raw[1024..1536].copy_from_slice(header.as_bytes());
    assert!(std::str::from_utf8(&path.as_bytes()[..100]).is_err());
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw).unwrap();
    let package = encoder.finish().unwrap();
    assert_eq!(
        unpack("tar_gz", &package, &path, 16384).unwrap()[0].relative_path,
        path
    );
}

#[test]
fn complete_archive_tree_preserves_empty_and_implicit_parent_directories() {
    use codeguard_runtime::unpack_package_archive_tree;
    let tar = tar_gz(&[("empty/", b"", b'5'), ("bin/tool", b"tool", b'0')]);
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .add_directory("empty/", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer
        .start_file("bin/tool", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"tool").unwrap();
    let zip = writer.finish().unwrap().into_inner();
    for (format, bytes) in [("tar_gz", tar), ("zip", zip)] {
        let tree = unpack_package_archive_tree(
            &bytes,
            &ArchiveUnpackRequest {
                format,
                entrypoint: "bin/tool",
                package_sha256: hash(&bytes),
                entrypoint_sha256: hash(b"tool"),
                max_unpacked_bytes: 16384,
            },
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(tree.directories, vec!["bin", "empty"]);
        assert_eq!(tree.files.len(), 1);
        assert_eq!(unpack(format, &bytes, "bin/tool", 16384).unwrap().len(), 1);
    }
}
#[test]
fn implicit_directory_case_conflicts_cannot_hide_inside_different_file_names() {
    let bytes = zip(&[("Bin/a", b"a"), ("bin/tool", b"tool")]);
    assert_eq!(
        unpack("zip", &bytes, "bin/tool", 16384).unwrap_err(),
        "bundle_path_conflict"
    );
}
