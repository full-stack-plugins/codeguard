"""只读核对K07内存Schema白名单与所有实际引用，不安装或访问外部资源。"""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "crates/codeguard-cli/src/engineering_projection/schema_sources.rs"
text = SOURCE.read_text(encoding="utf-8")
entries = re.findall(r'"([^"\n]+\.schema\.json)"\s*=>\s*Some\(include_bytes!\(\s*"([^"\n]+)"\s*\)\)', text)
assert entries and len(dict(entries)) == len(entries), "empty or duplicate Schema sources"
resources = dict(entries)
for name, include in entries:
    assert include == "../../../../schemas/" + name, "source escapes fixed schema directory"
    assert "/" not in name and "\\" not in name and ".." not in name
ids = dict(re.findall(r'\(\s*"([^"\n]+)"\s*,\s*"([^"\n]+\.schema\.json)"\s*,?\s*\)', text.split("const IDS:")[1]))
for identity, name in ids.items():
    assert name in resources
    assert json.loads((ROOT / "schemas" / name).read_text(encoding="utf-8"))["$id"] == identity

def references(value):
    if isinstance(value, dict):
        if "$ref" in value and not value["$ref"].startswith("#"):
            yield value["$ref"].split("#")[0]
        for child in value.values():
            yield from references(child)
    elif isinstance(value, list):
        for child in value:
            yield from references(child)

pending = [f"check-feedback-v0.{n}.schema.json" for n in range(38, 74) if (ROOT / "schemas" / f"check-feedback-v0.{n}.schema.json").exists()]
pending.extend(["check-aborted-v0.23.schema.json", "task-verification-preview-v0.40.schema.json", "java-p3c-file-feedback-v0.2.schema.json"])
seen = set()
while pending:
    name = pending.pop()
    name = ids.get(name, name)
    if name in seen:
        continue
    assert name in resources, f"unresolved/unlisted Schema: {name}"
    seen.add(name)
    pending.extend(references(json.loads((ROOT / "schemas" / name).read_text(encoding="utf-8"))))
assert seen == set(resources), f"source whitelist drift: {seen ^ set(resources)}"
print(f"K07 compiled Schema closure PASS: {len(seen)} files; no external retrieval")
