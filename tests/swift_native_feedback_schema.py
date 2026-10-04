"""开发验收：Swift 独立入口的真实 native/WASM 输出及拒绝边界。"""
import copy,json,unittest
from pathlib import Path
from jsonschema import Draft202012Validator
from referencing import Registry,Resource
ROOT=Path(__file__).resolve().parents[1]
class SwiftNativeFeedbackSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schemas={p.name:json.loads(p.read_text()) for p in (ROOT/'schemas').glob('*.schema.json')}
        cls.registry=Registry().with_resources([(n,Resource.from_contents(s)) for n,s in cls.schemas.items()]+[(s['$id'],Resource.from_contents(s)) for s in cls.schemas.values() if '$id' in s])
        cls.reports={n:v['report'] for n,v in json.loads((ROOT/'tests/acceptance/evidence/swift-native-single-file-2026-10-04.json').read_text())['reports'].items()}
    def validator(self,name='swift-lint-feedback-v0.1.schema.json'):return Draft202012Validator(self.schemas[name],registry=self.registry)
    def test_actual_reports(self):
        for n,r in self.reports.items():
            with self.subTest(n=n):self.validator().validate(r)
    def test_native_error_identity_and_positions_cannot_be_forged(self):
        for field,value in [('version','Apple Swift 6.3'),('reason','swift_native_parse_no_diagnostics')]:
            r=copy.deepcopy(self.reports['bad_parameter']);r['native'][field]=value;self.assertFalse(self.validator().is_valid(r))
        for field,value in [('rule_id','swift.type.error'),('line',0),('column',0)]:
            r=copy.deepcopy(self.reports['bad_parameter']);r['native']['diagnostics'][0][field]=value;self.assertFalse(self.validator().is_valid(r))
    def test_stale_input_and_selected_native_failures_cannot_keep_candidate_or_positions(self):
        r=copy.deepcopy(self.reports['bad_parameter']);r['input_stable']=False;self.assertFalse(self.validator().is_valid(r))
        r=copy.deepcopy(self.reports['good']);r['syntax_precheck']=self.reports['missing_clean']['syntax_precheck'];self.assertFalse(self.validator().is_valid(r))
    def test_hidden_recovery_cannot_remove_native_requirement_or_grant_approval(self):
        r=copy.deepcopy(self.reports['missing_hidden']);r['setup']['native_confirmation_required']=False;self.assertFalse(self.validator().is_valid(r))
        for field,value in [('coverage_proven',True),('delivery_decision','allow'),('authority','trusted')]:
            r=copy.deepcopy(self.reports['good']);r[field]=value;self.assertFalse(self.validator().is_valid(r))
        self.assertFalse(self.validator('kotlin-lint-feedback-v0.1.schema.json').is_valid(self.reports['good']))
if __name__=='__main__':unittest.main()
