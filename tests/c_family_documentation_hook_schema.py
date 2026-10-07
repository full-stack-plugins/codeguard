"""C-family修复Hook开发报告校验；不授予生产资格。"""
import copy
import hashlib
import json
from pathlib import Path
from jsonschema import Draft202012Validator
from referencing import Registry, Resource
root=Path(__file__).resolve().parents[1]
registry=Registry()
schemas={}
for path in (root/'schemas').glob('*.schema.json'):
    data=json.loads(path.read_text());Draft202012Validator.check_schema(data);schemas[path.name]=data
    for key in [path.name,path.as_uri(),data.get('$id',path.name)]:registry=registry.with_resource(key,Resource.from_contents(data))
validator=Draft202012Validator(schemas['hook-execution-feedback-v0.30.schema.json'],registry=registry)
count=0
sample=None
for mode in ['default','wasm']:
    evidence=json.loads((root/f'tests/acceptance/evidence/c-family-documentation-hook-{mode}.json').read_text())
    assert evidence['qualification']=='not_granted'
    assert evidence['test_source_sha256']==hashlib.sha256((root/'crates/codeguard-cli/tests/c_family_documentation_hook.rs').read_bytes()).hexdigest()
    for case in evidence['cases']:
        for task in case['tasks']:
            for key in ['present','absent','wrong_tool']:
                validator.validate(task[key]);count+=1
            sample=task['present']
negative=[]
for field,value in [('task_closure','closed'),('detailed_contract_qualification','production_ready'),('documentation_rule_source','trusted'),('documentation_standard','c17')]:
    forged=copy.deepcopy(sample);forged['local_feedback'][field]=value;negative.append(forged)
forged=copy.deepcopy(sample);forged['local_feedback']['documentation_input_current']=False;negative.append(forged)
forged=copy.deepcopy(sample);forged['local_feedback']['event_persisted']=False;negative.append(forged)
for field,value in [
    ('documentation_standard', 'c11' if sample['local_feedback']['documentation_standard']=='c++17' else 'c++17'),
    ('documentation_rule_source', 'native_clang_warning' if sample['local_feedback']['documentation_rule_source']=='codeguard_structural_policy' else 'codeguard_structural_policy'),
]:
    forged=copy.deepcopy(sample);forged['local_feedback'][field]=value;negative.append(forged)
for forged in negative:assert list(validator.iter_errors(forged))
result={'evidence_kind':'development_c_documentation_hook_schema','qualification':'not_granted','reports_valid':count,'negative_cases_rejected':len(negative),'schemas_valid':len(schemas)}
(root/'tests/acceptance/evidence/c-family-documentation-hook-schema.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
