"""开发验收：Swift 实际 parser Hook 报告与 CLI 宿主适配摘要。"""
import copy,json,unittest
from pathlib import Path
from jsonschema import Draft202012Validator
from referencing import Registry,Resource
ROOT=Path(__file__).resolve().parents[1]
class SwiftHookFeedbackSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schemas={p.name:json.loads(p.read_text()) for p in (ROOT/'schemas').glob('*.schema.json')}
        cls.registry=Registry().with_resources([(n,Resource.from_contents(s)) for n,s in cls.schemas.items()]+[(s['$id'],Resource.from_contents(s)) for s in cls.schemas.values() if '$id' in s])
        cls.reports={n:v['report'] for n,v in json.loads((ROOT/'tests/acceptance/evidence/swift-native-hook-2026-10-05.json').read_text())['reports'].items()}
    def validator(self,name='hook-execution-feedback-v0.12.schema.json'):return Draft202012Validator(self.schemas[name],registry=self.registry)
    def test_actual_hook_reports_and_old_consumer_rejection(self):
        for n in ['good','bad','missing']:
            r=self.reports[n];self.validator().validate(r);self.assertFalse(self.validator('hook-execution-feedback-v0.11.schema.json').is_valid(r))
    def test_no_fake_task_or_native_position_after_stale_input(self):
        r=copy.deepcopy(self.reports['bad']);r['local_feedback']['swift_lint']['files'][0]['current']=False;self.assertFalse(self.validator().is_valid(r))
        r=copy.deepcopy(self.reports['bad']);r['local_feedback']['swift_lint']['files'][0]['task_id']='CG-B-'+'a'*32;self.assertFalse(self.validator().is_valid(r))
    def test_no_native_message_or_unbounded_context(self):
        for n in ['claude_adapter_bad','claude_adapter_good']:
            text=self.reports[n]['hookSpecificOutput']['additionalContext'];self.assertLessEqual(len(text),1200);self.assertIn('原生任务同步尚未接线',text);self.assertIn('交付未评估',text);self.assertNotIn('expected type',text)
    def test_soft_feedback_cannot_claim_delivery_or_host_enforcement(self):
        for field,value in [('delivery_decision','allow'),('host_blocking_verified',True),('soft_result_reused',True)]:
            r=copy.deepcopy(self.reports['good']);r[field]=value;self.assertFalse(self.validator().is_valid(r))
if __name__=='__main__':unittest.main()
