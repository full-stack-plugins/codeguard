"""验证C/C++编辑文档Hook候选报告；不授予生产资格。"""
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
validator=Draft202012Validator(schemas['hook-execution-feedback-v0.31.schema.json'],registry=registry)
count=0
negative=0
for mode in ['default','wasm']:
    for language in ['c','cpp']:
        evidence=json.loads((root/f'tests/acceptance/evidence/c-family-edit-{mode}-{language}.json').read_text())
        assert evidence['qualification']=='not_granted'
        assert evidence['test_source_sha256']==hashlib.sha256((root/'crates/codeguard-cli/tests/c_family_documentation_edit_hook.rs').read_bytes()).hexdigest()
        assert all(isinstance(ids,list) and ids for ids in evidence['stable_task_ids'])
        report=evidence['report'];validator.validate(report);count+=1
        for field,value in [('coverage_proven',True),('delivery_decision','passed')]:
            forged=copy.deepcopy(report);forged['local_feedback'][field]=value
            assert list(validator.iter_errors(forged));negative+=1
        forged=copy.deepcopy(report);forged['local_feedback']['c_family_documentation'][language]['language']='cpp' if language=='c' else 'c'
        assert list(validator.iter_errors(forged));negative+=1
        forged=copy.deepcopy(report);forged['local_feedback']['c_family_documentation'][language]['standard']='c++17' if language=='c' else 'c11'
        assert list(validator.iter_errors(forged));negative+=1
result={'evidence_kind':'development_c_family_documentation_edit_schema','qualification':'not_granted','reports_valid':count,'negative_cases_rejected':negative,'schemas_valid':len(schemas)}
(root/'tests/acceptance/evidence/c-family-edit-schema.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
