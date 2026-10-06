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
historical = subprocess.check_output(["git", "ls-tree", "-r", "--name-only", "f6cc1bc", "schemas"], cwd=root, text=True).splitlines()
for path in historical:
    assert (root / path).read_bytes() == subprocess.check_output(["git", "show", f"f6cc1bc:{path}"], cwd=root), path
validate = lambda name, value: Draft202012Validator(schemas[name], registry=registry).validate(value)
count = 0
first = None
for name in ["c-family-structure-workbench-native.json", "c-family-structure-workbench-native-wasm.json"]:
    evidence = json.loads((root / "tests/acceptance/evidence" / name).read_text())
    assert evidence["qualification"] == "not_granted" and len(evidence["cases"]) == 2
    assert evidence["test_source_sha256"] == hashlib.sha256((root / "crates/codeguard-cli/tests/c_family_structure_workbench.rs").read_bytes()).hexdigest()
    for case in evidence["cases"]:
        assert len(case["packets"]) == 5
        for packet in case["packets"]:
            validate("clang-documentation-structure-workbench-observation-v0.1.schema.json", packet)
        for key in ["first", "repeated", "moved", "restored", "clean"]:
            report = case[key]
            validate("c-family-comments-feedback-v0.7.schema.json", report)
            validate("repair-brief-preview-v0.31.schema.json", report["structural_next"])
            first = first or report["structural_next"]
            count += 1
        validate("task-show-preview-v0.5.schema.json", case["stale"])
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
    assert list(Draft202012Validator(schemas["repair-brief-preview-v0.31.schema.json"], registry=registry).iter_errors(forged))
result = {"evidence_kind": "development_structure_workbench_schema", "qualification": "not_granted",
          "schemas_valid": len(schemas), "historical_schemas_unchanged": len(historical),
          "feedback_and_brief_pairs_valid": count, "task_show_valid": 4, "structural_packets_valid": 20, "negative_cases_rejected": len(negative)}
(root / "tests/acceptance/evidence/c-family-structure-workbench-schema.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result))
