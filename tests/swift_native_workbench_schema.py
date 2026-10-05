"""开发验收：Swift 首次原生任务、历史来源和复检反馈的实际协议。"""
import copy,json,unittest
from pathlib import Path
from jsonschema import Draft202012Validator
from referencing import Registry,Resource
ROOT=Path(__file__).resolve().parents[1]
class SwiftNativeWorkbenchSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schemas={p.name:json.loads(p.read_text()) for p in (ROOT/'schemas').glob('*.schema.json')}
        cls.registry=Registry().with_resources([(n,Resource.from_contents(s)) for n,s in cls.schemas.items()]+[(s['$id'],Resource.from_contents(s)) for s in cls.schemas.values() if '$id' in s])
        cls.evidence=json.loads((ROOT/'tests/acceptance/evidence/swift-native-workbench-2026-10-05.json').read_text());cls.reports={n:v['report'] for n,v in cls.evidence['reports'].items()}
    def validator(self,name):return Draft202012Validator(self.schemas[name],registry=self.registry)
    def test_actual_reports_and_stable_task(self):
        mapping={'first':'check-feedback-v0.44.schema.json','repeat':'check-feedback-v0.44.schema.json','next':'repair-brief-preview-v0.11.schema.json','hook':'hook-execution-feedback-v0.13.schema.json','verify_bad':'task-verification-preview-v0.18.schema.json','verify_fixed':'task-verification-preview-v0.18.schema.json','next_after_fix':'repair-brief-preview-v0.6.schema.json'}
        for n,s in mapping.items():self.validator(s).validate(self.reports[n])
        for o in self.evidence['origins']:self.validator('syntax-confirmation-observation-v0.5.schema.json').validate(o)
        self.assertEqual(self.reports['next_after_fix']['repair_brief']['task_id'],self.evidence['task_id'])
        self.assertEqual(self.reports['verify_fixed']['observation'],'candidate_absent_unverified_policy')
    def test_old_consumers_reject_new_sources(self):
        for n,s in [('first','check-feedback-v0.43.schema.json'),('next','repair-brief-preview-v0.6.schema.json'),('hook','hook-execution-feedback-v0.12.schema.json'),('verify_bad','task-verification-preview-v0.15.schema.json')]:self.assertFalse(self.validator(s).is_valid(self.reports[n]))
    def test_native_origin_cannot_forge_grammar_or_language(self):
        r=copy.deepcopy(self.reports['verify_bad']);r['native_scan']['original_report']['grammar_sha256']='a'*64;self.assertFalse(self.validator('task-verification-preview-v0.18.schema.json').is_valid(r))
        o=copy.deepcopy(self.evidence['origins'][0]);o['language']='kotlin';self.assertFalse(self.validator('syntax-confirmation-observation-v0.5.schema.json').is_valid(o))
    def test_zero_diagnostics_cannot_grant_delivery_or_hide_history(self):
        r=copy.deepcopy(self.reports['verify_fixed']);r['delivery_decision']='allow';self.assertFalse(self.validator('task-verification-preview-v0.18.schema.json').is_valid(r))
        self.assertTrue(self.reports['verify_fixed']['event_persisted']);self.assertEqual(self.evidence['final_state'],'open')
if __name__=='__main__':unittest.main()
