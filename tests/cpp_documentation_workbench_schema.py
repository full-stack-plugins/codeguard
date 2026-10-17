"""验证C++成员文档工作台实际CLI证据；不授予可信关闭或生产资格。"""
import copy
import hashlib
import json
from pathlib import Path
from jsonschema import Draft202012Validator
from referencing import Registry, Resource

root = Path(__file__).resolve().parents[1]
registry = Registry()
schemas = {}
for path in (root / 'schemas').glob('*.schema.json'):
    data = json.loads(path.read_text())
    Draft202012Validator.check_schema(data)
    schemas[path.name] = data
    for key in [path.name, path.as_uri(), data.get('$id', path.name)]:
        registry = registry.with_resource(key, Resource.from_contents(data))
first_validator = Draft202012Validator(schemas['c-family-comments-feedback-v0.9.schema.json'], registry=registry)
verify_validator = Draft202012Validator(schemas['task-verification-preview-v0.37.schema.json'], registry=registry)
count = negative = 0
for mode in ['default', 'wasm']:
    evidence = json.loads((root / f'tests/acceptance/evidence/cpp17-member-workbench-{mode}.json').read_text())
    assert evidence['qualification'] == 'not_granted'
    assert evidence['test_source_sha256'] == hashlib.sha256((root / 'crates/codeguard-cli/tests/cpp_documentation_workbench.rs').read_bytes()).hexdigest()
    first = evidence['first']
    first_validator.validate(first)
    count += 1
    assert first['structural_workbench']['task_ids'] == [evidence['stable_task_id']]
    for field, value in [('coverage_proven', True), ('detailed_contract_qualification', 'granted')]:
        forged = copy.deepcopy(first)
        forged[field] = value
        assert list(first_validator.iter_errors(forged))
        negative += 1
    for name, observation in [('present', 'still_present'), ('repaired', 'candidate_absent_unverified_policy'), ('recurrence', 'still_present')]:
        report = evidence[name]
        verify_validator.validate(report)
        count += 1
        assert report['task_id'] == evidence['stable_task_id']
        assert report['observation'] == observation and report['event_persisted']
        assert report['native_scan']['standard'] == 'c++17'
        assert report['native_scan']['input_stable']
        assert report['native_scan']['source_sha256'] == evidence['fixed_source_sha256' if name == 'repaired' else 'initial_source_sha256']
        for native, field, value in [(False, 'delivery_decision', 'passed'), (True, 'coverage_proven', True)]:
            forged = copy.deepcopy(report)
            target = forged['native_scan'] if native else forged
            target[field] = value
            assert list(verify_validator.iter_errors(forged))
            negative += 1
result = {'qualification': 'not_granted', 'scope': 'cpp_member_workbench_schema_only', 'reports_valid': count, 'negative_cases_rejected': negative, 'schemas_valid': len(schemas)}
(root / 'tests/acceptance/evidence/cpp17-member-workbench-schema.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
