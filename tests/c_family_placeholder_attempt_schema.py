"""核验实际占位尝试任务指引与预算；协议通过不等于生产资格。"""
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
schema = json.loads((root / "schemas/repair-brief-preview-v0.35.schema.json").read_text())
validator = Draft202012Validator(schema["properties"]["repair_brief"], registry=registry)
verify = Draft202012Validator(json.loads((root / "schemas/task-verification-preview-v0.38.schema.json").read_text()), registry=registry)
show = Draft202012Validator(json.loads((root / "schemas/task-show-preview-v0.8.schema.json").read_text()), registry=registry)
assert len(sys.argv) == 3
count = rejected = 0
for argument, language in zip(sys.argv[1:], ["c", "cpp"]):
    evidence = json.loads(Path(argument).read_text())
    for key in ["initial", "exhausted"]:
        brief = evidence[key]
        validator.validate(brief)
        assert brief["checker_id"] == f"{language}.clang.documentation_placeholder"
        count += 1
    for key in ["initial_show", "exhausted_show"]:
        show.validate(evidence[key])
        assert evidence[key]["next_actions"][0] == evidence[key]["task"]["recheck_argv"]
        count += 1
    assert evidence["initial"]["disposition"] == "actionable"
    exhausted = evidence["exhausted"]
    assert exhausted["allowed_paths"] == [] and exhausted["placeholder_positions"] == []
    assert exhausted["history"]["no_progress_count"] == 2
    assert exhausted["reason_code"] == "no_progress_budget_exhausted"
    for report in evidence["rechecks"]:
        verify.validate(report)
        assert report["observation"] == "still_present" and report["event_persisted"]
        count += 1
    for field, value in [("allowed_paths", ["api.c"]), ("placeholder_positions", evidence["initial"]["placeholder_positions"]),
                         ("authority", "trusted"), ("extra", True)]:
        mutation = copy.deepcopy(exhausted)
        mutation[field] = value
        assert not validator.is_valid(mutation), field
        rejected += 1
    mutation = copy.deepcopy(evidence["initial"])
    mutation["placeholder_rule_id"] = "codeguard.documentation.function_structure_required"
    assert not validator.is_valid(mutation)
    rejected += 1
print(json.dumps({"actual_reports": count, "rejected_mutations": rejected,
                  "qualification": "not_granted"}, ensure_ascii=False))
