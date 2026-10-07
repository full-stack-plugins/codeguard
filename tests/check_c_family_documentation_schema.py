"""开发报告协议/源码身份检查；不授予生产资格。"""
import copy
import hashlib
import json
import subprocess
from pathlib import Path
from jsonschema import Draft202012Validator
from referencing import Registry, Resource

root=Path(__file__).resolve().parents[1]
registry=Registry()
schemas={}
for path in sorted((root/'schemas').glob('*.schema.json')):
    data=json.loads(path.read_text())
    Draft202012Validator.check_schema(data)
    schemas[path.name]=data
    for key in [path.name,path.as_uri(),data.get('$id',path.name)]:
        registry=registry.with_resource(key,Resource.from_contents(data))
def validate(name,value):
    Draft202012Validator(schemas[name],registry=registry).validate(value)
old=subprocess.check_output(['git','ls-tree','-r','--name-only','cb0b42d','schemas'],cwd=root,text=True).splitlines()
for path in old:
    assert (root/path).read_bytes()==subprocess.check_output(['git','show',f'cb0b42d:{path}'],cwd=root),path
count=0
scans=0
child_reports=0
sample=None
source_sha=hashlib.sha256((root/'crates/codeguard-cli/tests/check_c_family_comments.rs').read_bytes()).hexdigest()
for mode in ['default','wasm']:
    evidence=json.loads((root/f'tests/acceptance/evidence/check-c-family-documentation-{mode}.json').read_text())
    assert evidence['qualification']=='not_granted'
    assert '结构缺口 1 项' in evidence['human']
    assert evidence['test_source_sha256']==source_sha
    assert len(evidence['binary_sha256'])==64
    for key in ['first','repeated','selected','unbound','missing_context','missing_tool','deadline','mutation','cancel','limit']:
        report=evidence[key]
        validate('check-feedback-v0.72.schema.json',report)
        count+=1
        sample=sample or report
        for language,scan in report['native_results']['c_family_comments'].items():
            validate('c-family-documentation-scan-v0.1.schema.json',scan)
            scans+=1
            for row in scan['files']:
                f=row['feedback']
                if f:
                    validate(f"c-family-comments-feedback-v{f['schema_version'][:-2]}.schema.json",f)
                    child_reports+=1
    validate('check-aborted-v0.22.schema.json',evidence['aborted_builder'])
    assert evidence['sarif']['runs'][0]['properties']['structuralPolicyFindingCount']==1
    assert evidence['sarif']['runs'][0]['properties']['nativeFindingCount']==0
    assert evidence['sarif']['runs'][0]['invocations'][0]['executionSuccessful'] is False
negative=[]
for field,value in [('authority','trusted'),('delivery_decision','allow'),('exit_code',0),('required_obligations',[]),('schema_version','1.0.0')]:
    forged=copy.deepcopy(sample);forged[field]=value;negative.append(forged)
for field,value in [('coverage_proven',True),('project_configuration','configured'),('file_limit',128),('standard','c++17')]:
    forged=copy.deepcopy(sample);forged['native_results']['c_family_comments']['c'][field]=value;negative.append(forged)
forged=copy.deepcopy(sample)
forged['native_results']['c_family_comments']['c']['scope_stable']=False
forged['native_results']['c_family_comments']['c']['local_scan_complete']=True
negative.append(forged)
forged=copy.deepcopy(sample)
forged['native_results']['c_family_comments']['c']['unobserved_count']=1
forged['native_results']['c_family_comments']['c']['local_scan_complete']=True
negative.append(forged)
for forged in negative:
    assert list(Draft202012Validator(schemas['check-feedback-v0.72.schema.json'],registry=registry).iter_errors(forged)),forged
result={'evidence_kind':'development_check_c_documentation_schema','qualification':'not_granted','schemas_valid':len(schemas),'historical_schemas_unchanged':len(old),'check_reports_valid':count,'language_scans_valid':scans,'child_reports_valid':child_reports,'controlled_aborted_builders_valid':2,'negative_cases_rejected':len(negative),'test_source_sha256':source_sha}
(root/'tests/acceptance/evidence/check-c-family-documentation-schema.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result))
