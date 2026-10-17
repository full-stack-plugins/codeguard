"""开发期实际 Rust Hook/任务协议验收；Python 不参与产品检测运行时。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class RustNativeHookSchema(unittest.TestCase):
    def reports(self):
        return json.loads((ROOT / 'tests/acceptance/evidence/rust-native-hook-controlled-2026-10-06.json').read_text())

    def test_actual_public_chain_reports(self):
        self.assertEqual(len(self.reports()), 7)
        names = {'hook_execution_feedback':'hook-execution-feedback', 'repair_brief_preview':'repair-brief-preview', 'task_verification_preview':'task-verification-preview'}
        for r in self.reports():
            name = names[r['report_type']]+'-v'+r['schema_version'][:-2]+'.schema.json'
            validator(name).validate(r)
            bad = copy.deepcopy(r); bad['delivery_decision'] = 'allow'
            self.assertFalse(validator(name).is_valid(bad))

    def test_native_forged_identity_status_and_positions_are_rejected(self):
        r = next(r for r in self.reports() if r.get('local_feedback',{}).get('rust_syntax',{}).get('local_parse_complete') is True)
        v = validator('hook-execution-feedback-v0.24.schema.json')
        for key,value in [('status','completed'),('version','rustfmt arbitrary'),('edition',None),('edition_context',None),('tool_sha256',None),('reason','rustfmt_native_parse_no_diagnostics')]:
            bad=copy.deepcopy(r); bad['local_feedback']['rust_syntax']['files'][0]['native'][key]=value
            self.assertFalse(v.is_valid(bad),key)
        bad=copy.deepcopy(r);bad['local_feedback']['rust_syntax']['files'][0]['current']=False
        self.assertFalse(v.is_valid(bad))
        bad=copy.deepcopy(r);bad['local_feedback']['rust_syntax']['files'][0]['native']['diagnostics'][0]['column']=1
        self.assertFalse(v.is_valid(bad))
        bad=copy.deepcopy(r);bad['local_feedback']['rust_syntax']['files'][0]['path']='../outside.rs'
        self.assertFalse(v.is_valid(bad))

if __name__ == '__main__':
    unittest.main()
