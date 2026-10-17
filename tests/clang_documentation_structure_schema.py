"""开发阶段原生AST结构协议回归，不属于Codeguard运行内核或生产资格。"""
import copy
import json
import hashlib
import subprocess
from pathlib import Path

from jsonschema import Draft202012Validator

root = Path(__file__).resolve().parents[1]
schema_path = root / "schemas/clang-function-documentation-structure-v0.1.schema.json"
validator = Draft202012Validator(json.loads(schema_path.read_text()))
Draft202012Validator.check_schema(validator.schema)
historical = subprocess.check_output(
    ["git", "ls-tree", "-r", "--name-only", "cd0da4f", "schemas"], cwd=root, text=True
).splitlines()
for name in historical:
    expected = subprocess.check_output(["git", "show", f"cd0da4f:{name}"], cwd=root)
    assert (root / name).read_bytes() == expected, name
count = 0
first = None
for name in ["clang-documentation-structure-native.json", "clang-documentation-structure-native-wasm.json"]:
    evidence = json.loads((root / "tests/acceptance/evidence" / name).read_text())
    assert evidence["qualification"] == "not_granted"
    for field, source in [("adapter_source_sha256", "crates/codeguard-adapters/src/clang_documentation_ast.rs"), ("test_source_sha256", "crates/codeguard-cli/tests/clang_documentation_structure_native.rs")]:
        assert evidence[field] == hashlib.sha256((root / source).read_bytes()).hexdigest()
    for case in evidence["cases"]:
        validator.validate(case["observation"])
        first = first or case["observation"]
        count += 1
assert count == 60
negative = []
for field, value in [("qualification", "granted"), ("coverage_proven", True),
                     ("semantic_accuracy", "proven"), ("errors_and_behavior", "complete"),
                     ("authority", "trusted"), ("extra", "unexpected")]:
    forged = copy.deepcopy(first)
    forged[field] = value
    negative.append(forged)
for key, value in [("missing_components", []), ("offset_byte", -1), ("line", 0)]:
    forged = copy.deepcopy(first)
    forged["functions"][0][key] = value
    negative.append(forged)
for forged in negative:
    assert list(validator.iter_errors(forged)), forged
result = {"evidence_kind": "development_ast_schema_regression", "qualification": "not_granted",
          "historical_schemas_unchanged": len(historical), "native_observations_valid": count,
          "negative_cases_rejected": len(negative)}
(root / "tests/acceptance/evidence/clang-documentation-structure-schema.json").write_text(
    json.dumps(result, indent=2) + "\n"
)
print(json.dumps(result))
