"""结构任务开发协议回归；公开任务不等于可信关闭或生产验收。"""
import copy
import json
import hashlib
import subprocess
from pathlib import Path
from jsonschema import Draft202012Validator
from referencing import Registry, Resource

root = Path(__file__).resolve().parents[1]
registry = Registry()
schemas = {}
for path in sorted((root / "schemas").glob("*.schema.json")):
    data = json.loads(path.read_text())
    Draft202012Validator.check_schema(data)
    schemas[path.name] = data
    resource = Resource.from_contents(data)
    for key in [path.name, path.as_uri(), data.get("$id", path.name)]:
        registry = registry.with_resource(key, resource)
historical = subprocess.check_output(["git", "ls-tree", "-r", "--name-only", "688d6b9", "schemas"], cwd=root, text=True).splitlines()
for path in historical:
    assert (root / path).read_bytes() == subprocess.check_output(["git", "show", f"688d6b9:{path}"], cwd=root), path
validate = lambda name, value: Draft202012Validator(schemas[name], registry=registry).validate(value)
count = 0
first = None
for name in ["c-family-structure-workbench-native.json", "c-family-structure-workbench-native-wasm.json"]:
    evidence = json.loads((root / "tests/acceptance/evidence" / name).read_text())
    assert evidence["qualification"] == "not_granted" and len(evidence["cases"]) == 2
    assert evidence["test_source_sha256"] == hashlib.sha256((root / "crates/codeguard-cli/tests/c_family_structure_workbench.rs").read_bytes()).hexdigest()
    for case in evidence["cases"]:
        assert len(case["packets"]) == 11
        for packet in case["packets"]:
            validate("clang-documentation-structure-task-recheck-v0.1.schema.json" if packet["report_type"]=="clang_documentation_structure_task_recheck" else "clang-documentation-structure-workbench-observation-v0.1.schema.json", packet)
        for attempt in case["attempts"]:
            validate("task-verification-preview-v0.37.schema.json",attempt["recheck"])
        validate("repair-brief-preview-v0.33.schema.json",case["repeat_budget"]["structural_next"])
        for key in ["verify_present", "verify_absent"]:
            validate("task-verification-preview-v0.37.schema.json",case[key])
        for key in ["first", "repeated", "moved", "restored", "clean"]:
            report = case[key]
            validate("c-family-comments-feedback-v0.9.schema.json", report)
            validate("repair-brief-preview-v0.33.schema.json", report["structural_next"])
            first = first or report["structural_next"]
            count += 1
        validate("task-show-preview-v0.7.schema.json", case["stale"])
for name in ["c-family-structure-recheck-native.json","c-family-structure-recheck-native-wasm.json"]:
    evidence=json.loads((root/"tests/acceptance/evidence"/name).read_text())
    assert evidence["qualification"]=="not_granted"
    for field, path in [("helper_source_sha256","crates/codeguard-cli/src/c_family_structure_task_recheck.rs"),("test_source_sha256","crates/codeguard-cli/src/c_family_structure_recheck_tests.rs")]:
        assert evidence[field]==hashlib.sha256((root/path).read_bytes()).hexdigest()
    validate("c-family-comments-feedback-v0.9.schema.json",evidence["initial_feedback"])
    assert evidence["unsupported_standard_exit_code"]==2
    for field in ["first","clean","expired","syntax_error"]:
        validate("clang-documentation-structure-task-recheck-v0.1.schema.json",evidence[field])
negative = []
for field, value in [("rule_source", "native_clang_warning"), ("structural_rule_id", "clang.warn_doc_missing"),
                     ("authority", "trusted"), ("history", {"task_verify": "complete"})]:
    forged = copy.deepcopy(first)
    forged["repair_brief"][field] = value
    negative.append(forged)
forged = copy.deepcopy(first)
forged["repair_brief"]["structural_positions"] = []
negative.append(forged)
for forged in negative:
    assert list(Draft202012Validator(schemas["repair-brief-preview-v0.33.schema.json"], registry=registry).iter_errors(forged))
result = {"evidence_kind": "development_structure_workbench_schema", "qualification": "not_granted",
          "schemas_valid": len(schemas), "historical_schemas_unchanged": len(historical),
          "feedback_and_brief_pairs_valid": count, "task_show_valid": 4, "structural_packets_valid": 44, "helper_native_reports_valid":8,"task_verifications_valid":16,"workspace_feedback_valid":2,"unsupported_standard_argument_cases":2,"negative_cases_rejected": len(negative)}
(root / "tests/acceptance/evidence/c-family-structure-workbench-schema.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result))
