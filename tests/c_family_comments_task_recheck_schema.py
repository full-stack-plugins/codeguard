"""开发期封闭协议回放；不进入Rust运行时或授予生产资格。"""
import copy
import json
from pathlib import Path
import subprocess
import sys

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

root = Path(__file__).resolve().parents[1]
documents = {p.name: json.loads(p.read_text()) for p in (root / "schemas").glob("*.schema.json")}
registry = Registry().with_resources((d["$id"], Resource.from_contents(d)) for d in documents.values() if "$id" in d)
new = {"c-family-comments-feedback-v0.3.schema.json", "repair-brief-preview-v0.29.schema.json", "clang-documentation-task-recheck-v0.1.schema.json", "task-verification-preview-v0.36.schema.json"}
historical = 0
for name, document in documents.items():
    Draft202012Validator.check_schema(document)
    if name not in new:
        original = subprocess.run(["git", "show", f"HEAD:schemas/{name}"], cwd=root, capture_output=True, check=True).stdout
        assert original == (root / "schemas" / name).read_bytes(), name
        historical += 1
mapping = {"c_family_comments_feedback": "c-family-comments-feedback-v0.3.schema.json", "repair_brief_preview": "repair-brief-preview-v0.29.schema.json", "clang_documentation_workbench_observation": "clang-documentation-workbench-observation-v0.1.schema.json", "clang_documentation_task_recheck":"clang-documentation-task-recheck-v0.1.schema.json", "task_verification_preview":"task-verification-preview-v0.36.schema.json"}
rows = [json.loads(p.read_text()) for p in Path(sys.argv[1]).glob("*.json")]
counts = {kind: 0 for kind in mapping}
for row in rows:
    kind = row["report_type"]
    if kind == "task_verification_preview" and row["schema_version"] != "0.36.0": continue
    Draft202012Validator(documents[mapping[kind]], registry=registry).validate(row)
    counts[kind] += 1
assert all(counts.values()), counts
feedback = next(r for r in rows if r["report_type"] == "c_family_comments_feedback" and r["next"] is not None)
brief = feedback["next"]
packet = next(r for r in rows if r["report_type"] == "clang_documentation_workbench_observation" and r["local_scan_complete"])
negative = []
for source in [feedback, brief, packet]:
    for key, value in [("authority", "trusted"), ("delivery_decision", "allow")]:
        mutated = copy.deepcopy(source)
        mutated[key] = value
        negative.append(mutated)
for key, value in [("coverage_proven", True), ("standard", "c++17"), ("local_scan_complete", False), ("extra_authorization", True)]:
    mutated = copy.deepcopy(packet)
    mutated[key] = value
    negative.append(mutated)
mutated = copy.deepcopy(brief)
mutated["repair_brief"]["observation_status"] = "candidate_absent_unverified"
negative.append(mutated)
mutated = copy.deepcopy(feedback)
mutated["verification_command"].append("--disable-checks")
negative.append(mutated)
recheck = next(r for r in rows if r["report_type"] == "clang_documentation_task_recheck")
verification = next(r for r in rows if r["report_type"] == "task_verification_preview" and r["schema_version"] == "0.36.0")
for source, key, value in [(recheck, "coverage_proven",True),(recheck,"extra_authorization",True),(recheck,"task_kind","closed"),(verification,"observation","resolved"),(verification,"delivery_decision","allow")]:
    mutated=copy.deepcopy(source);mutated[key]=value;negative.append(mutated)
mutated=copy.deepcopy(verification);mutated["native_scan"]["input_stable"]=False;mutated["observation"]="still_present";negative.append(mutated)
for row in negative:
    validator = Draft202012Validator(documents[mapping[row["report_type"]]], registry=registry)
    assert not validator.is_valid(row), row
old = Draft202012Validator(documents["c-family-comments-feedback-v0.1.schema.json"], registry=registry)
assert not old.is_valid(feedback)
assert not Draft202012Validator(documents["c-family-comments-feedback-v0.2.schema.json"], registry=registry).is_valid(feedback)
assert not Draft202012Validator(documents["repair-brief-preview-v0.28.schema.json"], registry=registry).is_valid(brief)
result = {"qualification": "not_granted", "evidence_kind": "development_schema_regression", "schemas_valid": len(documents), "historical_schemas_unchanged": historical, "reports_valid": counts, "negative_cases_rejected": len(negative), "old_consumer_rejects_new_version": True}
if len(sys.argv) > 2:
    Path(sys.argv[2]).write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result))
