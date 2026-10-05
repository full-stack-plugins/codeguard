"""开发期 Clippy 修复反馈协议验收；不参与产品检测运行时。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class ClippyHookFeedbackSchema(unittest.TestCase):
    def reports(self):
        return json.loads((ROOT/'tests/acceptance/evidence/clippy-hook-controlled-2026-10-06.json').read_text())

    def test_actual_diagnostic_absence_and_stale_reports(self):
        v=validator('hook-execution-feedback-v0.26.schema.json')
        self.assertEqual({r['local_feedback']['native_confirmation_status'] for r in self.reports()}, {'diagnostics_observed','completed','stale'})
        for report in self.reports():
            v.validate(report)
            self.assertEqual(report['local_feedback']['checker_id'],'rust.cargo_clippy')
            self.assertEqual(report['delivery_decision'],'not_evaluated')
        self.assertFalse(validator('hook-execution-feedback-v0.25.schema.json').is_valid(self.reports()[0]))

    def test_actual_installed_cargo_reports(self):
        reports=json.loads((ROOT/'tests/acceptance/evidence/clippy-hook-native-2026-10-06.json').read_text())
        self.assertEqual(len(reports),2)
        self.assertEqual({r['local_feedback']['native_confirmation_status'] for r in reports}, {'diagnostics_observed','completed'})
        for r in reports:
            validator('hook-execution-feedback-v0.26.schema.json').validate(r)
            self.assertTrue(r['local_feedback']['event_persisted'])
            self.assertEqual(r['delivery_decision'],'not_evaluated')

    def test_forged_scope_status_and_coordinates_are_rejected(self):
        r=next(r for r in self.reports() if r['local_feedback']['native_confirmation_status']=='diagnostics_observed')
        v=validator('hook-execution-feedback-v0.26.schema.json')
        for key,value in [('native_confirmation_status','stale'),('native_confirmation_status','completed'),('native_column_unit','utf8_byte'),('checker_id','syntax.native_confirmation'),('native_confirmation_reason','clippy_confirmation_no_task_diagnostics')]:
            bad=copy.deepcopy(r);bad['local_feedback'][key]=value
            self.assertFalse(v.is_valid(bad),key)
        for key,value in [('line',0),('rule_id','rust.syntax'),('column',1)]:
            bad=copy.deepcopy(r);bad['local_feedback']['native_diagnostic_positions'][0][key]=value
            self.assertFalse(v.is_valid(bad),key)
        bad=copy.deepcopy(r);bad['delivery_decision']='allow'
        self.assertFalse(v.is_valid(bad))

if __name__=='__main__':
    unittest.main()
