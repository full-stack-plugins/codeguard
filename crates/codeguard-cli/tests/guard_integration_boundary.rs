use codeguard_cli::guard_integration::{
    projection::{ProtectedMapping, project},
    reader::read_native,
    scope::FrozenObligations,
};
use guardengine::{GuardContract, GuardSubject};
use serde_json::json;
use std::{collections::BTreeMap, io::Write};
#[path = "support/guard_integration.rs"]
mod fixture;

#[test]
fn library_projection_probe() {
    let bytes = serde_json::to_vec(&fixture::valid()).unwrap();
    let invocation = fixture::invocation();
    let obligations = FrozenObligations::new(BTreeMap::from([(
        "python/app/lint/ruff".into(),
        vec!["src/main.py".into()],
    )]))
    .unwrap();
    let contract: GuardContract = serde_json::from_value(json!({"apiVersion":guardengine::API_VERSION,"kind":"GuardContract","metadata":{"id":"c","revision":"1"},"spec":{"rules":[{"id":"r","enforcement":"advise","assertion":{"type":"forbid_relation","subject":"code","predicate":"has","object":"gap"}}]}})).unwrap();
    let mapping = ProtectedMapping::parse(br#"{"version":"codeguard.mapping/v1alpha1","entries":[{"source":{"kind":"finding","tool_id":"ruff","native_rule_id":"F401"},"rule_id":"r"},{"source":{"kind":"gap","detail":"native profile unqualified"},"rule_id":"r"}]}"#).unwrap();
    println!("GUARD_BOUNDARY_BEGIN");
    std::io::stdout().flush().unwrap();
    let mut prior = None;
    for _ in 0..10 {
        let evidence = read_native(&bytes, &invocation).unwrap();
        let projected = project(
            &evidence,
            &obligations,
            &mapping,
            &contract,
            GuardSubject {
                id: "repo".into(),
                snapshot_digest: format!("sha256:{}", "a".repeat(64)),
            },
        )
        .unwrap();
        assert_eq!(projected.domain_bytes(), bytes);
        let report = serde_json::to_vec(projected.report()).unwrap();
        if let Some(prior) = &prior {
            assert_eq!(&report, prior);
        }
        prior = Some(report);
    }
    println!("GUARD_BOUNDARY_END");
    std::io::stdout().flush().unwrap();
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires Linux strace; run explicitly for boundary qualification"]
fn linux_syscalls_show_no_projection_io_or_native_query_adapter_activity() {
    use std::{fs, process::Command};
    let root = std::env::temp_dir().join(format!("cg-boundary-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let project = root.join("project");
    for dir in ["src", ".codeguard", ".git/refs/heads"] {
        fs::create_dir_all(project.join(dir)).unwrap();
    }
    let files = [
        "src/main.py",
        ".codeguard/events.jsonl",
        ".git/index",
        ".git/refs/heads/main",
    ];
    for name in files {
        fs::write(project.join(name), format!("unchanged {name}\n")).unwrap();
    }
    let library_trace = root.join("library.trace");
    let library = Command::new("strace")
        .args(["-f", "-e", "trace=%file,%process,%network,write", "-o"])
        .arg(&library_trace)
        .arg(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "library_projection_probe",
            "--nocapture",
            "--test-threads=1",
        ])
        .current_dir(&project)
        .output()
        .expect("strace must be installed for this explicit test");
    assert!(
        library.status.success(),
        "{}",
        String::from_utf8_lossy(&library.stderr)
    );
    let trace = fs::read_to_string(&library_trace).unwrap();
    let begin = trace.find("GUARD_BOUNDARY_BEGIN").expect("begin marker");
    let end = trace[begin..].find("GUARD_BOUNDARY_END").unwrap() + begin;
    for line in trace[begin..end].lines().skip(1) {
        let call = line
            .split('(')
            .next()
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap_or("");
        assert!(
            ![
                "open", "stat", "access", "exec", "clone", "fork", "socket", "connect", "send",
                "recv", "unlink", "rename", "mkdir", "readlink", "getdents"
            ]
            .iter()
            .any(|prefix| call.contains(prefix)),
            "unexpected projection syscall: {line}"
        );
    }
    let native_trace = root.join("native.trace");
    let native = Command::new("strace")
        .args(["-f", "-e", "trace=%file,%process,%network", "-o"])
        .arg(&native_trace)
        .arg(env!("CARGO_BIN_EXE_codeguard"))
        .args(["--version", "--format=json"])
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(native.status.success());
    assert!(native.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&native.stdout).unwrap();
    assert_eq!(value["report_type"], "version");
    let trace = fs::read_to_string(&native_trace).unwrap();
    assert_eq!(
        trace
            .lines()
            .filter(|line| line.contains("execve("))
            .count(),
        1
    );
    for prohibited in [
        "clone(",
        "clone3(",
        "fork(",
        "vfork(",
        "socket(",
        "connect(",
        "guardengine",
        "guard-project",
        ".codeguard",
        ".git/",
        "src/main.py",
    ] {
        assert!(
            !trace.contains(prohibited),
            "unselected query accessed {prohibited}"
        );
    }
    for name in files {
        assert_eq!(
            fs::read_to_string(project.join(name)).unwrap(),
            format!("unchanged {name}\n")
        );
    }
    println!(
        "Linux syscall traces: {}; {}",
        library_trace.display(),
        native_trace.display()
    );
    // Traces are retained outside the source tree for review; no product state is written.
}
