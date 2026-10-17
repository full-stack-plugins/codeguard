"""验证真实公开占位任务复检报文；协议合法不授予生产资格。"""
import copy
import json
import sys
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

root = Path(__file__).resolve().parents[1]
registry = Registry()
for path in sorted((root / "schemas").glob("*.schema.json")):
    schema = json.loads(path.read_text())
    resource = Resource.from_contents(schema)
    registry = registry.with_resource(path.name, resource)
    if "$id" in schema:
        registry = registry.with_resource(schema["$id"], resource)
schema = json.loads((root / "schemas/task-verification-preview-v0.39.schema.json").read_text())
Draft202012Validator.check_schema(schema)
validator = Draft202012Validator(schema, registry=registry)
assert len(sys.argv) == 3, "显式传入实际C、C++公开复检JSON"
count = rejected = 0
for argument, language in zip(sys.argv[1:], ["c", "cpp"]):
    reports = json.loads(Path(argument).read_text())
    assert len(reports) == 2
    for report, outcome in zip(reports, ["still_present", "candidate_absent_unverified_policy"]):
        validator.validate(report)
        assert report["observation"] == outcome
        assert report["event_persisted"] is True
        assert report["native_scan"]["language"] == language
        assert report["native_scan"]["input_stable"] is True
        assert report["native_scan"]["native"]["tool_sha256"]
        count += 1
        for field, value in [("task_rule", "native.clang.warning"), ("coverage_proven", True),
                             ("authority", "trusted"), ("extra", True)]:
            mutation = copy.deepcopy(report)
            mutation["native_scan"][field] = value
            assert not validator.is_valid(mutation), field
            rejected += 1
        mutation = copy.deepcopy(report)
        mutation["native_scan"]["input_stable"] = False
        assert not validator.is_valid(mutation), "失稳输入不能仍称候选消失或存在"
        rejected += 1
        mutation = copy.deepcopy(report)
        mutation["native_scan"]["placeholders"]["semantic_accuracy"] = "accepted"
        assert not validator.is_valid(mutation), "不得伪造语义精度验收"
        rejected += 1
print(json.dumps({"actual_reports": count, "rejected_mutations": rejected,
                  "qualification": "not_granted"}, ensure_ascii=False))
