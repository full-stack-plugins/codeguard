use codeguard_runtime::{ProcessOutcome, ProcessSpec, Termination, run_process};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn oracle() -> Value {
    serde_json::from_slice(include_bytes!(
        "../../../tests/acceptance/evidence/cfquery-postgres-oracle-2026-10-06.json"
    ))
    .unwrap()
}

#[test]
fn native_sql_dialect_counterevidence_does_not_relabel_generic_pending_case() {
    let evidence = oracle();
    assert_eq!(evidence["authority"], "development_only");
    assert_eq!(evidence["grammar_qualified"], false);
    assert_eq!(evidence["delivery_decision"], "not_evaluated");
    let cases = evidence["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 2);
    for case in cases {
        assert_eq!(case["dialect"], "postgresql");
        assert_eq!(case["query_execution"], "not_executed_prepare_only");
        assert_eq!(
            case["source_sha256"],
            format!(
                "{:x}",
                Sha256::digest(case["source"].as_str().unwrap().as_bytes())
            )
        );
    }
    assert_eq!(cases[0]["native_valid"], true);
    assert_eq!(cases[1]["native_valid"], false);
    assert_eq!(cases[1]["sqlstate"], "42601");
    let corpus: Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/grammar_regression_v0_2.json"
    ))
    .unwrap();
    let generic = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "cfquery-missing_select_list")
        .unwrap();
    assert_eq!(generic["source_sha256"], cases[0]["source_sha256"]);
    assert_eq!(generic["label"], "pending");
    assert_eq!(generic["cohort"], "provisional_syntax");
}

/// 独占且无网络的原生SQL语法夹具；来源：OpenSpec S14.17明确方言原生对照。
struct PostgresFixture {
    tool: PathBuf,
    id: Option<String>,
}
impl PostgresFixture {
    fn run(&self, args: &[&str]) -> ProcessOutcome {
        let env = [
            "PATH",
            "HOME",
            "DOCKER_HOST",
            "DOCKER_CONTEXT",
            "DOCKER_CONFIG",
        ]
        .into_iter()
        .filter_map(|k| std::env::var_os(k).map(|v| (OsString::from(k), v)))
        .collect::<BTreeMap<_, _>>();
        run_process(
            &ProcessSpec {
                executable: self.tool.clone(),
                args: args.iter().map(OsString::from).collect(),
                cwd: std::env::temp_dir(),
                env,
                stdin: None,
                deadline: Instant::now() + Duration::from_secs(15),
                output_limit_bytes: 64 * 1024,
            },
            &AtomicBool::new(false),
        )
    }
}
impl Drop for PostgresFixture {
    fn drop(&mut self) {
        if let Some(id) = &self.id {
            let _ = self.run(&["rm", "--force", id]);
        }
    }
}

#[test]
#[ignore = "requires explicit CODEGUARD_DOCKER_TOOL and already cached pinned PostgreSQL image; never pulls"]
fn isolated_native_postgres_confirms_empty_projection_and_distinct_counterexample() {
    let tool =
        PathBuf::from(std::env::var_os("CODEGUARD_DOCKER_TOOL").expect("explicit Docker path"));
    assert!(tool.is_absolute());
    let mut fixture = PostgresFixture { tool, id: None };
    let evidence = oracle();
    let image = evidence["image_id"].as_str().unwrap();
    let existing = fixture.run(&["image", "inspect", "--format", "{{.Id}}", image]);
    assert_eq!(existing.termination, Termination::Exited(0));
    assert_eq!(String::from_utf8(existing.stdout).unwrap().trim(), image);
    let name = format!(
        "codeguard-pg-oracle-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let started = fixture.run(&[
        "run",
        "--detach",
        "--rm",
        "--pull=never",
        "--network",
        "none",
        "--name",
        &name,
        "--tmpfs",
        "/var/lib/postgresql:rw",
        "-e",
        "PGDATA=/var/lib/postgresql/data",
        "-e",
        "POSTGRES_HOST_AUTH_METHOD=trust",
        image,
    ]);
    assert_eq!(
        started.termination,
        Termination::Exited(0),
        "{:?}",
        started.stderr
    );
    let id = String::from_utf8(started.stdout).unwrap().trim().to_owned();
    assert!(id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()));
    fixture.id = Some(id.clone());
    let mut ready = false;
    for _ in 0..40 {
        if fixture
            .run(&["exec", &id, "pg_isready", "-U", "postgres"])
            .termination
            == Termination::Exited(0)
        {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    assert!(ready, "isolated server readiness exceeded");
    let version = fixture.run(&["exec", &id, "postgres", "--version"]);
    assert_eq!(version.termination, Termination::Exited(0));
    assert_eq!(
        String::from_utf8(version.stdout).unwrap().trim(),
        evidence["server_version"].as_str().unwrap()
    );
    for case in evidence["cases"].as_array().unwrap() {
        let sql = format!(
            "CREATE TEMP TABLE users (id integer); PREPARE codeguard_case AS {};",
            case["source"].as_str().unwrap()
        );
        let result = fixture.run(&[
            "exec",
            &id,
            "psql",
            "-X",
            "-U",
            "postgres",
            "-v",
            "ON_ERROR_STOP=1",
            "-v",
            "VERBOSITY=verbose",
            "-c",
            &sql,
        ]);
        assert_eq!(
            result.termination,
            Termination::Exited(case["native_exit_code"].as_i64().unwrap() as i32)
        );
        if case["native_valid"] == false {
            assert!(String::from_utf8(result.stderr).unwrap().contains("42601"));
        }
    }
    let removed = fixture.run(&["rm", "--force", &id]);
    assert_eq!(removed.termination, Termination::Exited(0));
    fixture.id = None;
}

#[cfg(all(feature = "wasm-precheck", unix))]
#[test]
fn current_cfquery_worker_preserves_real_native_disagreement_without_fake_clean() {
    let evidence = oracle();
    let directory = std::env::temp_dir()
        .canonicalize()
        .unwrap()
        .join(format!("cg-cfquery-dialect-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    for case in evidence["cases"].as_array().unwrap() {
        let source = directory.join(format!("{}.sql", case["id"].as_str().unwrap()));
        std::fs::write(&source, case["source"].as_str().unwrap()).unwrap();
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(["grammar", "probe", "cfquery"])
            .arg(&source)
            .arg("--format=json")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["source_sha256"], case["source_sha256"]);
        if case["native_valid"] == true {
            assert_eq!(report["recoveries"], serde_json::json!([]));
        }
        assert_eq!(report["status"], "incomplete");
        assert_eq!(report["grammar_qualified"], false);
        assert_eq!(report["delivery_decision"], "not_evaluated");
    }
    std::fs::remove_dir_all(directory).unwrap();
}
