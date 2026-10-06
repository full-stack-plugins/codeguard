#![cfg(unix)]

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

struct Project(PathBuf);

impl Project {
    fn new(rule: &str, source: &str) -> Self {
        let root = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("cg-ruff-doc-{}-{rule}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("app.py"), source).unwrap();
        fs::write(root.join("ruff.toml"), format!("preview = true\n[lint]\nselect = ['{rule}']\n[lint.pydocstyle]\nconvention = 'google'\n[lint.pydoclint]\nignore-one-line-docstrings = false\n")).unwrap();
        let project = Self(root);
        project.run(&[
            "init",
            project.0.to_str().unwrap(),
            "--apply",
            "--format=json",
        ]);
        project
    }

    fn run(&self, args: &[&str]) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_codeguard"))
            .args(args)
            .env_remove("CODEGUARD_TIMEOUT")
            .output()
            .unwrap();
        assert!(
            matches!(output.status.code(), Some(0 | 3)),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn check(&self, tool: &str) -> Value {
        self.run(&[
            "check",
            "python",
            self.0.to_str().unwrap(),
            "--ruff-tool",
            tool,
            "--format=json",
        ])
    }

    fn comments(&self, tool: &str) -> Value {
        self.run(&[
            "comments",
            "python",
            self.0.to_str().unwrap(),
            "--ruff-tool",
            tool,
            "--format=json",
        ])
    }

    fn verify(&self, id: &str, tool: &str) -> Value {
        self.run(&[
            "task",
            "verify",
            id,
            self.0.to_str().unwrap(),
            "--ruff-tool",
            tool,
            "--format=json",
        ])
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// 真实已有 Ruff 执行；本测试不安装工具，不用运输夹具冒充原生语义。
#[test]
#[ignore = "需要已有 Ruff 0.16.8，显式 CODEGUARD_RUFF_BIN；不安装或启用项目外规则"]
fn actual_pydoclint_rules_reach_comments_tasks_and_original_rechecks() {
    let tool = PathBuf::from(std::env::var("CODEGUARD_RUFF_BIN").unwrap())
        .canonicalize()
        .unwrap();
    let tool = tool.to_str().unwrap();
    let binary_sha256 = format!(
        "{:x}",
        Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())
    );
    let cases = [
        (
            "DOC102",
            "def f(x):\n    \"\"\"Describe operation.\n\n    Args:\n        x: Actual value.\n        phantom: Nonexistent value.\n    \"\"\"\n    print(x)\n",
            "def f(x):\n    \"\"\"Describe operation.\n\n    Args:\n        x: Actual value.\n    \"\"\"\n    print(x)\n",
        ),
        (
            "DOC201",
            "def f() -> int:\n    \"\"\"Compute one.\n\n    Details of operation.\n    \"\"\"\n    return 1\n",
            "def f() -> int:\n    \"\"\"Compute one.\n\n    Returns:\n        int: Constant one.\n    \"\"\"\n    return 1\n",
        ),
        (
            "DOC202",
            "def f():\n    \"\"\"Print one.\n\n    Returns:\n        int: Incorrect return claim.\n    \"\"\"\n    print(1)\n",
            "def f():\n    \"\"\"Print one without returning a value.\"\"\"\n    print(1)\n",
        ),
        (
            "DOC402",
            "def f() -> Iterator[int]:\n    \"\"\"Generate one.\n\n    Details of generation.\n    \"\"\"\n    yield 1\n",
            "def f() -> Iterator[int]:\n    \"\"\"Generate one.\n\n    Yields:\n        int: Constant one.\n    \"\"\"\n    yield 1\n",
        ),
        (
            "DOC403",
            "def f():\n    \"\"\"Print one.\n\n    Yields:\n        int: Incorrect generator claim.\n    \"\"\"\n    print(1)\n",
            "def f():\n    \"\"\"Print one without generating values.\"\"\"\n    print(1)\n",
        ),
        (
            "DOC501",
            "def f(x: int) -> None:\n    \"\"\"Reject negative values.\n\n    Details of validation.\n    \"\"\"\n    if x < 0:\n        raise ValueError('negative')\n",
            "def f(x: int) -> None:\n    \"\"\"Reject negative values.\n\n    Raises:\n        ValueError: When the value is negative.\n    \"\"\"\n    if x < 0:\n        raise ValueError('negative')\n",
        ),
        (
            "DOC502",
            "def f():\n    \"\"\"Print one.\n\n    Raises:\n        ValueError: Incorrect explicit exception claim.\n    \"\"\"\n    print(1)\n",
            "def f():\n    \"\"\"Print one.\"\"\"\n    print(1)\n",
        ),
    ];
    let mut evidence = Vec::new();
    for (rule, source, repaired) in cases {
        let project = Project::new(rule, source);
        let direct = project.comments(tool);
        assert_eq!(direct["report_type"], "python_comments_feedback");
        assert_eq!(direct["local_scan_complete"], true, "{direct}");
        assert!(
            direct["documentation_findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|finding| finding["rule_id"] == rule),
            "{direct}"
        );
        assert_eq!(direct["detailed_contract_qualification"], "not_granted");
        let first = project.check(tool);
        let files = first["native_results"]["python_lint"]["files"]
            .as_array()
            .unwrap();
        let finding = files
            .iter()
            .flat_map(|file| file["findings"].as_array().unwrap())
            .find(|finding| finding["rule_id"] == rule)
            .unwrap_or_else(|| panic!("{rule}: {first}"));
        assert!(
            first["category_candidates"].as_array().unwrap().iter().any(
                |candidate| candidate["language"] == "python"
                    && candidate["category"] == "comments"
                    && candidate["status"] == "observed_unverified"
            ),
            "{rule}: {first}"
        );
        assert_ne!(finding["rule_summary"], "查看原生规则说明及私有诊断详情");
        assert_eq!(
            finding["repair_hint"]["status"],
            if rule == "DOC502" {
                "investigation_required"
            } else {
                "bounded_repair_candidate"
            }
        );
        let id = finding["finding_id"].as_str().unwrap();
        let task = fs::read_to_string(project.0.join(format!(".codeguard/tasks/{id}.md"))).unwrap();
        assert!(task.contains(if rule == "DOC502" {
            "隐式异常"
        } else {
            "实际"
        }));
        let next = project.run(&["next", project.0.to_str().unwrap(), "--format=json"]);
        assert_eq!(next["repair_brief"]["native_rule_id"], rule);
        assert!(
            next["repair_brief"]["step"]
                .as_str()
                .unwrap()
                .contains(if rule == "DOC502" {
                    "隐式异常"
                } else {
                    "实际"
                })
        );
        let repeated = project.check(tool);
        assert!(
            repeated["native_results"]["python_lint"]["files"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|file| file["findings"].as_array().unwrap())
                .any(|finding| finding["rule_id"] == rule && finding["finding_id"] == id)
        );
        let present = project.verify(id, tool);
        assert_eq!(present["observation"], "still_present", "{rule}: {present}");
        let suppressed_source =
            source.replace("    \"\"\"\n", &format!("    \"\"\"  # noqa: {rule}\n"));
        fs::write(project.0.join("app.py"), suppressed_source).unwrap();
        let suppressed = project.verify(id, tool);
        assert_eq!(
            suppressed["observation"], "suppression_requires_review",
            "{rule}: {suppressed}"
        );
        fs::write(project.0.join("app.py"), repaired).unwrap();
        let absent = project.verify(id, tool);
        assert_eq!(
            absent["observation"], "candidate_absent_unverified_policy",
            "{rule}: {absent}"
        );
        let fact: Value = serde_json::from_slice(
            &fs::read(
                project
                    .0
                    .join(format!(".codeguard/findings/{id}/finding.json")),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(fact["state"], "open");
        evidence.push(json!({"codeguard_binary_sha256":binary_sha256,"rule":rule,"direct_comments":direct,"first":first,"next":next,"present":present,"suppressed":suppressed,"repeated":repeated,"absent":absent,"fact_state":fact["state"]}));
    }
    assert_eq!(
        binary_sha256,
        format!(
            "{:x}",
            Sha256::digest(fs::read(env!("CARGO_BIN_EXE_codeguard")).unwrap())
        )
    );
    if let Ok(path) = std::env::var("CODEGUARD_TEST_RUFF_DOC_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}

#[test]
#[ignore = "需要已有 Ruff 0.16.8，显式 CODEGUARD_RUFF_BIN；保留原生豁免和 DOC502 局限"]
fn actual_native_exemptions_and_implicit_exception_require_no_invented_violation() {
    let tool = PathBuf::from(std::env::var("CODEGUARD_RUFF_BIN").unwrap())
        .canonicalize()
        .unwrap();
    let tool = tool.to_str().unwrap();
    let legal = [
        "def f():\n    \"\"\"Return one.\"\"\"\n    return 1\n",
        "def f():\n    \"\"\"Yield one.\"\"\"\n    yield 1\n",
        "def f():\n    \"\"\"Describe placeholder.\"\"\"\n    pass\n",
        "def f():\n    \"\"\"Describe optional result.\"\"\"\n    return None\n",
        "def f():\n    \"\"\"Describe empty generation.\"\"\"\n    yield None\n",
        "from abc import ABC, abstractmethod\n\nclass A(ABC):\n    @abstractmethod\n    def f(self):\n        \"\"\"Describe abstract contract.\"\"\"\n        pass\n",
    ];
    let project = Project::new("DOC", legal[0]);
    let mut evidence = Vec::new();
    for source in legal {
        fs::write(project.0.join("app.py"), source).unwrap();
        let report = project.check(tool);
        let files = report["native_results"]["python_lint"]["files"]
            .as_array()
            .unwrap();
        assert!(
            files.iter().all(|file| file["run_status"] == "passed"
                && file["findings"].as_array().unwrap().is_empty()),
            "{source}: {report}"
        );
        assert!(
            !report["category_candidates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|candidate| candidate["category"] == "comments"
                    && candidate["status"] == "observed_unverified")
        );
        evidence.push(json!({"case":"native_exemption","report":report}));
    }
    let abstract_source = "from abc import ABC, abstractmethod\n\nclass A(ABC):\n    @abstractmethod\n    def f(self):\n        \"\"\"Describe abstract contract.\"\"\"\n        return 1\n";
    fs::write(project.0.join("app.py"), abstract_source).unwrap();
    let abstract_report = project.check(tool);
    assert!(
        abstract_report["native_results"]["python_lint"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|file| file["findings"].as_array().unwrap())
            .any(|finding| finding["rule_id"] == "DOC201")
    );
    assert_eq!(
        fs::read_to_string(project.0.join("app.py")).unwrap(),
        abstract_source
    );
    evidence.push(
        json!({"case":"native_0_16_8_nonstub_abstract_method_boundary","report":abstract_report}),
    );
    fs::write(project.0.join("app.py"), "def f(value):\n    \"\"\"Divide one by the input.\n\n    Args:\n        value: Divisor.\n    Returns:\n        float: Reciprocal of the input.\n    Raises:\n        ZeroDivisionError: When the divisor is zero.\n    \"\"\"\n    return 1 / value\n").unwrap();
    let implicit = project.check(tool);
    let finding = implicit["native_results"]["python_lint"]["files"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|file| file["findings"].as_array().unwrap())
        .find(|finding| finding["rule_id"] == "DOC502")
        .unwrap();
    assert_eq!(finding["repair_hint"]["status"], "investigation_required");
    assert!(
        finding["repair_hint"]["step"]
            .as_str()
            .unwrap()
            .contains("不得自动删除")
    );
    evidence.push(json!({"case":"real_implicit_exception_convention_conflict","report":implicit}));
    fs::write(
        project.0.join("ruff.toml"),
        "preview=false\n[lint]\nselect=['F401']\n",
    )
    .unwrap();
    let direct_unconfigured = project.comments(tool);
    assert_eq!(direct_unconfigured["documentation_findings"], json!([]));
    assert_eq!(
        direct_unconfigured["documentation_rule_coverage"],
        "unverified"
    );
    assert_eq!(
        direct_unconfigured["detailed_contract_qualification"],
        "not_granted"
    );
    let unconfigured = project.check(tool);
    assert!(
        unconfigured["native_results"]["python_lint"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .all(|file| file["findings"].as_array().unwrap().is_empty())
    );
    evidence.push(json!({"case":"doc_not_selected_no_preview_injection","direct_comments":direct_unconfigured,"report":unconfigured}));
    if let Ok(path) = std::env::var("CODEGUARD_TEST_RUFF_DOC_EXEMPTION_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}
