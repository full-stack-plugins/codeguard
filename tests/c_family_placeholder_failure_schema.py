"""验证真实失败复检及权限撤回；本地观察不授予交付资格。"""
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
    Draft202012Validator.check_schema(schema)
    resource = Resource.from_contents(schema)
    registry = registry.with_resource(path.name, resource)
    if "$id" in schema:
        registry = registry.with_resource(schema["$id"], resource)
verify = Draft202012Validator(json.loads((root / "schemas/task-verification-preview-v0.39.schema.json").read_text()), registry=registry)
show = Draft202012Validator(json.loads((root / "schemas/task-show-preview-v0.9.schema.json").read_text()), registry=registry)
assert len(sys.argv) == 3
count = rejected = 0
for argument, language in zip(sys.argv[1:], ["c", "cpp"]):
    evidence = json.loads(Path(argument).read_text())
    for key in ["failed", "timed", "recovered"]:
        report = evidence[key]
        verify.validate(report)
        assert report["native_scan"]["language"] == language
        assert report["event_persisted"] is True
        assert report["observation"] == ("still_present" if key == "recovered" else "incomplete")
        count += 1
        if key == "recovered":
            continue
        assert report["native_scan"]["placeholders"] is None
        for field, value in [("observation", "candidate_absent_unverified_policy"), ("observation", "still_present"),
                             ("authority", "trusted"), ("extra", True)]:
            mutation = copy.deepcopy(report)
            mutation[field] = value
            assert not verify.is_valid(mutation), field
            rejected += 1
        mutation = copy.deepcopy(report)
        mutation["native_scan"]["placeholder_observation_status"] = "observed"
        assert not verify.is_valid(mutation), "失败不能伪造完整占位观察"
        rejected += 1
    show.validate(evidence["show"])
    count += 1
    assert evidence["show"]["task"]["reason_code"] == "native_placeholder_incomplete"
    mutation = copy.deepcopy(evidence["show"])
    mutation["task"]["allowed_paths"] = ["api.c"]
    assert not show.is_valid(mutation), "失败指引不能保持修改权限"
    rejected += 1
print(json.dumps({"actual_reports": count, "rejected_mutations": rejected,
                  "qualification": "not_granted"}, ensure_ascii=False))
