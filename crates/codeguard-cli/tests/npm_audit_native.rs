#![cfg(unix)]
use codeguard_adapters::{NpmAuditCommand, NpmLockedNode, parse_npm_audit_json};
use codeguard_runtime::{ProcessSpec, Termination, run_process_recorded};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
#[ignore = "requires existing explicit Node/npm 11.16.0; offline native empty/nonempty lock and missing lock"]
fn native_npm_offline_report_and_missing_lock_are_distinguished_without_installing() {
    use std::os::unix::fs::DirBuilderExt;
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-native-npm-{}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let fixture = Fixture(root);
    let package = fixture.0.join("package.json");
    let lock = fixture.0.join("package-lock.json");
    let package_bytes=br#"{"name":"cg-native-npm-fixture","version":"1.0.0","private":true,"scripts":{"preaudit":"touch SHOULD_NOT_EXIST"}}"#;
    let lock_bytes=br#"{"name":"cg-native-npm-fixture","version":"1.0.0","lockfileVersion":3,"requires":true,"packages":{"":{"name":"cg-native-npm-fixture","version":"1.0.0"}}}"#;
    std::fs::write(&package, package_bytes).unwrap();
    std::fs::write(&lock, lock_bytes).unwrap();
    let command = NpmAuditCommand {
        node: PathBuf::from(std::env::var_os("CODEGUARD_NODE_BIN").unwrap())
            .canonicalize()
            .unwrap(),
        entry: PathBuf::from(std::env::var_os("CODEGUARD_NPM_ENTRY").unwrap())
            .canonicalize()
            .unwrap(),
        cache: fixture.0.join("cache"),
        user_config: fixture.0.join("user.npmrc"),
        global_config: fixture.0.join("global.npmrc"),
    };
    std::fs::write(&command.user_config, "").unwrap();
    std::fs::write(&command.global_config, "").unwrap();
    let mut version_args = command.args().unwrap();
    version_args[2] = "--version".into();
    let version = run_process_recorded(
        &ProcessSpec {
            executable: command.node.clone(),
            args: version_args,
            cwd: fixture.0.clone(),
            env: BTreeMap::new(),
            stdin: None,
            deadline: Instant::now() + Duration::from_secs(90),
            output_limit_bytes: 1024 * 1024,
        },
        &AtomicBool::new(false),
        &fixture.0,
        "version.log",
    )
    .unwrap();
    assert_eq!(version.termination, Termination::Exited(0));
    assert_eq!(version.stdout, b"11.16.0\n");
    let nonempty_lock = br#"{"name":"cg-native-npm-fixture","version":"1.0.0","lockfileVersion":3,"requires":true,"packages":{"":{"name":"cg-native-npm-fixture","version":"1.0.0","dependencies":{"fixture-pkg":"1.2.3"}},"node_modules/fixture-pkg":{"version":"1.2.3","resolved":"https://registry.npmjs.org/fixture-pkg/-/fixture-pkg-1.2.3.tgz"}}}"#;
    let nonempty_package = br#"{"name":"cg-native-npm-fixture","version":"1.0.0","private":true,"dependencies":{"fixture-pkg":"1.2.3"},"scripts":{"preaudit":"touch SHOULD_NOT_EXIST"}}"#;
    for mode in ["empty", "nonempty", "missing"] {
        if mode == "nonempty" {
            std::fs::write(&lock, nonempty_lock).unwrap();
            std::fs::write(&package, nonempty_package).unwrap();
        }
        if mode == "missing" {
            std::fs::remove_file(&lock).unwrap();
        }
        let outcome = run_process_recorded(
            &ProcessSpec {
                executable: command.node.clone(),
                args: command.args().unwrap(),
                cwd: fixture.0.clone(),
                env: BTreeMap::new(),
                stdin: None,
                deadline: Instant::now() + Duration::from_secs(90),
                output_limit_bytes: 8 * 1024 * 1024,
            },
            &AtomicBool::new(false),
            &fixture.0,
            match mode {
                "empty" => "locked.log",
                "nonempty" => "nonempty.log",
                _ => "missing.log",
            },
        )
        .unwrap();
        let exit = match outcome.termination {
            Termination::Exited(code) => Some(code),
            _ => None,
        };
        let parsed = parse_npm_audit_json(&outcome.stdout, "11.16.0", "11.16.0", exit);
        if mode == "empty" {
            let observation = parsed.unwrap();
            assert!(observation.components.is_empty());
            assert_eq!(observation.native_dependency_total, 0);
            assert_eq!(std::fs::read(&lock).unwrap(), lock_bytes);
            let evidence = fixture.0.join("bound-evidence");
            std::fs::DirBuilder::new()
                .mode(0o700)
                .create(&evidence)
                .unwrap();
            use sha2::{Digest, Sha256};
            let paths = [
                &command.node,
                &command.entry,
                &package,
                &lock,
                &command.user_config,
                &command.global_config,
            ];
            let expected = paths
                .into_iter()
                .map(|path| {
                    (
                        path.clone(),
                        Sha256::digest(std::fs::read(path).unwrap()).into(),
                    )
                })
                .collect();
            let bound = codeguard_cli::npm_audit_probe::run_npm_audit_probe(
                &codeguard_cli::npm_audit_probe_request::NpmAuditProbeRequest {
                    command: NpmAuditCommand {
                        node: command.node.clone(),
                        entry: command.entry.clone(),
                        cache: command.cache.clone(),
                        user_config: command.user_config.clone(),
                        global_config: command.global_config.clone(),
                    },
                    cwd: fixture.0.clone(),
                    evidence_dir: evidence,
                    run_id: "bound".into(),
                    expected_version: "11.16.0".into(),
                    registry: None,
                    expected_sha256: expected,
                    deadline: Instant::now() + Duration::from_secs(45),
                },
                &AtomicBool::new(false),
            );
            assert!(bound.local_coherent, "{:?}", bound.reason);
            assert_eq!(bound.parsed.unwrap().advisory_coverage, "not_evaluated");
        } else if mode == "nonempty" {
            assert_eq!(NpmLockedNode::parse(nonempty_lock).unwrap().len(), 1);
            assert_eq!(std::fs::read(&lock).unwrap(), nonempty_lock);
            let observation = parsed.unwrap();
            assert_eq!(observation.native_dependency_total, 1);
            assert!(observation.components.is_empty());
            assert_eq!(observation.advisory_coverage, "not_evaluated");
            assert!(
                NpmLockedNode::bind(&NpmLockedNode::parse(nonempty_lock).unwrap(), &observation)
                    .unwrap()
                    .is_empty()
            );
        } else {
            assert_eq!(parsed.unwrap_err(), "npm_native_error");
            assert!(!lock.exists());
        }
        assert_eq!(
            std::fs::read(&package).unwrap(),
            if mode == "empty" {
                package_bytes.as_slice()
            } else {
                nonempty_package.as_slice()
            }
        );
        assert!(!fixture.0.join("node_modules").exists());
        assert!(!fixture.0.join("SHOULD_NOT_EXIST").exists());
    }
}

struct Registry {
    endpoint: String,
    stop: std::sync::Arc<AtomicBool>,
    fail: std::sync::Arc<AtomicBool>,
    requests: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Registry {
    fn start() -> Self {
        use std::io::{Read, Write};
        use std::sync::{Arc, Mutex, atomic::Ordering};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let fail = Arc::new(AtomicBool::new(false));
        let failing = fail.clone();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let observed = requests.clone();
        let thread = std::thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                let Ok((mut stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0u8; 4096];
                loop {
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => bytes.extend_from_slice(&buffer[..n]),
                    }
                    if bytes.len() > 1024 * 1024 {
                        break;
                    }
                    if let Some(end) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                        let length = headers.lines().find_map(|l| {
                            l.strip_prefix("content-length:")
                                .and_then(|n| n.trim().parse::<usize>().ok())
                        });
                        if length.is_some_and(|n| bytes.len() >= end + 4 + n)
                            || (headers.contains("transfer-encoding: chunked")
                                && bytes.ends_with(b"0\r\n\r\n"))
                            || (!headers.contains("transfer-encoding:") && length.is_none())
                        {
                            break;
                        }
                    }
                }
                let request = String::from_utf8_lossy(&bytes)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_owned();
                observed.lock().unwrap().push(request.clone());
                let (status, body) = if failing.load(Ordering::Relaxed) {
                    ("503 Service Unavailable", "{}".into())
                } else if request.starts_with("POST /-/npm/v1/security/advisories/bulk ") {
                    ("200 OK", serde_json::json!({"fixture-pkg":[{"id":123,"name":"fixture-pkg","title":"controlled advisory fixture","url":"https://example.invalid/advisory/123","severity":"high","vulnerable_versions":"<2.0.0","cwe":["CWE-20"],"cvss":{"score":7.5,"vectorString":null}}]}).to_string())
                } else if request.starts_with("GET /fixture-pkg ") {
                    ("200 OK", serde_json::json!({"name":"fixture-pkg","dist-tags":{"latest":"2.0.0"},"versions":{"1.2.3":{"name":"fixture-pkg","version":"1.2.3"},"2.0.0":{"name":"fixture-pkg","version":"2.0.0"}}}).to_string())
                } else {
                    ("404 Not Found", "{}".into())
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        Self {
            endpoint,
            stop,
            fail,
            requests,
            thread: Some(thread),
        }
    }
}
impl Drop for Registry {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

#[test]
#[ignore = "requires existing Node/npm 11.16.0; controlled loopback advisory is not a trusted database"]
fn native_npm_advisory_binds_to_locked_version_without_installing_or_claiming_coverage() {
    use std::os::unix::fs::DirBuilderExt;
    let registry = Registry::start();
    let root = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-npm-advisory-{}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&root)
        .unwrap();
    let fixture = Fixture(root);
    let package = br#"{"name":"fixture-app","version":"1.0.0","private":true,"dependencies":{"fixture-pkg":"1.2.3"},"scripts":{"preaudit":"touch SHOULD_NOT_EXIST"}}"#;
    let lock = br#"{"name":"fixture-app","version":"1.0.0","lockfileVersion":3,"requires":true,"packages":{"":{"name":"fixture-app","version":"1.0.0","dependencies":{"fixture-pkg":"1.2.3"}},"node_modules/fixture-pkg":{"version":"1.2.3"}}}"#;
    std::fs::write(fixture.0.join("package.json"), package).unwrap();
    std::fs::write(fixture.0.join("package-lock.json"), lock).unwrap();
    let command = NpmAuditCommand {
        node: PathBuf::from(std::env::var_os("CODEGUARD_NODE_BIN").unwrap())
            .canonicalize()
            .unwrap(),
        entry: PathBuf::from(std::env::var_os("CODEGUARD_NPM_ENTRY").unwrap())
            .canonicalize()
            .unwrap(),
        cache: fixture.0.join("cache"),
        user_config: fixture.0.join("user.npmrc"),
        global_config: fixture.0.join("global.npmrc"),
    };
    std::fs::write(&command.user_config, "").unwrap();
    std::fs::write(&command.global_config, "").unwrap();
    let result = run_process_recorded(
        &ProcessSpec {
            executable: command.node.clone(),
            args: command.args_for_registry(&registry.endpoint).unwrap(),
            cwd: fixture.0.clone(),
            env: BTreeMap::new(),
            stdin: None,
            deadline: Instant::now() + Duration::from_secs(45),
            output_limit_bytes: 8 * 1024 * 1024,
        },
        &AtomicBool::new(false),
        &fixture.0,
        "advisory.log",
    )
    .unwrap();
    assert_eq!(
        result.termination,
        Termination::Exited(1),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    let observation = parse_npm_audit_json(&result.stdout, "11.16.0", "11.16.0", Some(1))
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&result.stdout)));
    assert_eq!(observation.advisory_coverage, "not_evaluated");
    assert_eq!(observation.components.len(), 1);
    assert_eq!(observation.components[0].advisory_sources, vec![123]);
    let nodes = NpmLockedNode::parse(lock).unwrap();
    assert_eq!(
        NpmLockedNode::bind(&nodes, &observation).unwrap()[0].resolved_version,
        "1.2.3"
    );
    let requests = registry.requests.lock().unwrap();
    assert!(
        requests
            .iter()
            .any(|r| r.starts_with("POST /-/npm/v1/security/advisories/bulk "))
    );
    assert!(requests.iter().any(|r| r.starts_with("GET /fixture-pkg ")));
    assert!(requests.iter().all(
        |r| r.starts_with("POST /-/npm/v1/security/advisories/bulk ")
            || r.starts_with("GET /fixture-pkg ")
    ));
    assert_eq!(
        std::fs::read(fixture.0.join("package.json")).unwrap(),
        package
    );
    assert_eq!(
        std::fs::read(fixture.0.join("package-lock.json")).unwrap(),
        lock
    );
    assert!(!fixture.0.join("node_modules").exists());
    assert!(!fixture.0.join("SHOULD_NOT_EXIST").exists());
    drop(requests);
    let fixed_package = String::from_utf8_lossy(package).replace("1.2.3", "2.0.0");
    let fixed_lock = String::from_utf8_lossy(lock).replace("1.2.3", "2.0.0");
    std::fs::write(fixture.0.join("package.json"), &fixed_package).unwrap();
    std::fs::write(fixture.0.join("package-lock.json"), &fixed_lock).unwrap();
    let recheck = run_process_recorded(
        &ProcessSpec {
            executable: command.node.clone(),
            args: command.args_for_registry(&registry.endpoint).unwrap(),
            cwd: fixture.0.clone(),
            env: BTreeMap::new(),
            stdin: None,
            deadline: Instant::now() + Duration::from_secs(45),
            output_limit_bytes: 8 * 1024 * 1024,
        },
        &AtomicBool::new(false),
        &fixture.0,
        "fixed.log",
    )
    .unwrap();
    assert_eq!(recheck.termination, Termination::Exited(0));
    let fixed = parse_npm_audit_json(&recheck.stdout, "11.16.0", "11.16.0", Some(0)).unwrap();
    assert!(fixed.components.is_empty());
    assert_eq!(fixed.advisory_coverage, "not_evaluated");
    assert!(
        NpmLockedNode::bind(
            &NpmLockedNode::parse(fixed_lock.as_bytes()).unwrap(),
            &fixed
        )
        .unwrap()
        .is_empty()
    );
    assert_eq!(
        std::fs::read(fixture.0.join("package.json")).unwrap(),
        fixed_package.as_bytes()
    );
    assert_eq!(
        std::fs::read(fixture.0.join("package-lock.json")).unwrap(),
        fixed_lock.as_bytes()
    );
    assert!(!fixture.0.join("node_modules").exists());
    assert!(!fixture.0.join("SHOULD_NOT_EXIST").exists());
    registry
        .fail
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let failure = run_process_recorded(
        &ProcessSpec {
            executable: command.node.clone(),
            args: command.args_for_registry(&registry.endpoint).unwrap(),
            cwd: fixture.0.clone(),
            env: BTreeMap::new(),
            stdin: None,
            deadline: Instant::now() + Duration::from_secs(45),
            output_limit_bytes: 8 * 1024 * 1024,
        },
        &AtomicBool::new(false),
        &fixture.0,
        "unavailable.log",
    )
    .unwrap();
    let exit = match failure.termination {
        Termination::Exited(code) => Some(code),
        _ => None,
    };
    assert_eq!(
        parse_npm_audit_json(&failure.stdout, "11.16.0", "11.16.0", exit).unwrap_err(),
        "npm_native_error"
    );
}
