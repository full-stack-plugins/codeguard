"""开发验收：真实 Swift 项目局部观察与历史消费者边界。"""
import copy,json,unittest
from pathlib import Path
from jsonschema import Draft202012Validator
from referencing import Registry,Resource
ROOT=Path(__file__).resolve().parents[1]
class SwiftProjectFeedbackSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schemas={p.name:json.loads(p.read_text()) for p in (ROOT/'schemas').glob('*.schema.json')}
        cls.registry=Registry().with_resources([(n,Resource.from_contents(s)) for n,s in cls.schemas.items()]+[(s['$id'],Resource.from_contents(s)) for s in cls.schemas.values() if '$id' in s])
        cls.reports={n:v['report'] for n,v in json.loads((ROOT/'tests/acceptance/evidence/swift-native-project-2026-10-04.json').read_text())['reports'].items()}
    def validator(self,name='check-feedback-v0.43.schema.json'):return Draft202012Validator(self.schemas[name],registry=self.registry)
    def test_actual_reports_and_old_consumer_rejection(self):
        for r in self.reports.values():
            self.validator().validate(r)
            self.assertFalse(self.validator('check-feedback-v0.42.schema.json').is_valid(r))
    def test_stale_observation_cannot_retain_positions_or_recheck(self):
        r=copy.deepcopy(self.reports['bad_parameter']);r['native_results']['swift_lint']['files'][0]['current']=False
        self.assertFalse(self.validator().is_valid(r))
    def test_no_task_connection_can_be_forged(self):
        for field,value in [('task_id','CG-B-'+'a'*32),('task_sync_reason','approved')]:
            r=copy.deepcopy(self.reports['bad_parameter']);r['native_results']['swift_lint']['files'][0][field]=value;self.assertFalse(self.validator().is_valid(r))
    def test_partial_scan_cannot_be_complete_or_deliver(self):
        r=copy.deepcopy(self.reports['good']);r['native_results']['swift_lint']['unobserved_count']=1
        self.assertFalse(self.validator().is_valid(r))
        r=copy.deepcopy(self.reports['good']);r['delivery_decision']='allow';self.assertFalse(self.validator().is_valid(r))
if __name__=='__main__':unittest.main()
