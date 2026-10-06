"""开发阶段公开原生结构反馈协议回归；不授予生产资格。"""
import copy
import hashlib
import json
import subprocess
from pathlib import Path
from jsonschema import Draft202012Validator
from referencing import Registry, Resource

root = Path(__file__).resolve().parents[1]
registry = Registry()
schemas = {}
for path in sorted((root / "schemas").glob("*.schema.json")):
    document = json.loads(path.read_text())
    Draft202012Validator.check_schema(document)
    schemas[path.name] = document
    resource = Resource.from_contents(document)
    for key in [path.name, path.as_uri(), document.get("$id", path.name)]:
        registry = registry.with_resource(key, resource)
historic = subprocess.check_output(["git", "ls-tree", "-r", "--name-only", "294690b", "schemas"], cwd=root, text=True).splitlines()
for path in historic:
    assert (root / path).read_bytes() == subprocess.check_output(["git", "show", f"294690b:{path}"], cwd=root), path

def validator(version):
    return Draft202012Validator(schemas[f"c-family-comments-feedback-v{version}.schema.json"], registry=registry)

reports = []
for name in ["c-family-comments-structure-cli.json", "c-family-comments-structure-cli-wasm.json"]:
    evidence = json.loads((root / "tests/acceptance/evidence" / name).read_text())
    assert evidence["qualification"] == "not_granted"
    assert evidence["test_source_sha256"] == hashlib.sha256((root / "crates/codeguard-cli/tests/c_family_comments_structure_cli.rs").read_bytes()).hexdigest()
    assert len(evidence["cases"]) == 12
    reports.extend(case["report"] for case in evidence["cases"])
for directory in [Path("/tmp/cg-structure-public-default"), Path("/tmp/cg-structure-public-wasm")]:
    for path in directory.glob("*.json"):
        report = json.loads(path.read_text())
        if report.get("report_type") == "c_family_comments_feedback":
            reports.append(report)
for report in reports:
    version = report["schema_version"].removesuffix(".0")
    assert version in ["0.5", "0.6"]
    validator(version).validate(report)
    assert list(validator("0.1").iter_errors(report))
    assert list(validator("0.4").iter_errors(report))
first = next(r for r in reports if r["documentation_structure"]["status"] == "observed")
negative = []
for field, value in [("coverage_proven", True), ("detailed_contract_qualification", "granted"),
                     ("structural_task_workflow_status", "complete"), ("unexpected", True)]:
    forged = copy.deepcopy(first)
    forged[field] = value
    negative.append(forged)
for field, value in [("observation", None), ("native_raw_diagnostic_count", None),
                     ("reason", "clang_structure_unavailable")]:
    forged = copy.deepcopy(first)
    forged["documentation_structure"][field] = value
    negative.append(forged)
for field, value in [("semantic_accuracy", "proven"), ("coverage_proven", True)]:
    forged = copy.deepcopy(first)
    forged["documentation_structure"]["observation"][field] = value
    negative.append(forged)
for forged in negative:
    assert list(validator(first["schema_version"].removesuffix(".0")).iter_errors(forged))
result = {"evidence_kind": "development_public_structure_schema", "qualification": "not_granted",
          "schemas_valid": len(schemas), "historical_schemas_unchanged": len(historic),
          "feedback_valid": len(reports), "negative_cases_rejected": len(negative), "old_consumers_reject_new_versions": True}
(root / "tests/acceptance/evidence/c-family-comments-structure-schema.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result))
