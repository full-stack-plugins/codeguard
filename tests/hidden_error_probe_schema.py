"""不可定位解析错误的实际报告和反例；仅开发期协议验收。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class HiddenErrorProbe(unittest.TestCase):
    def test_actual_hidden_error_reports_require_native_confirmation(self):
        rows=json.loads((ROOT/'tests/acceptance/evidence/hidden-error-probe-2026-10-06.json').read_text())['reports']
        self.assertEqual(len(rows),3)
        for row in rows:
            validator('grammar-probe-v0.5.schema.json').validate(row)
            self.assertFalse(validator('grammar-probe-v0.1.schema.json').is_valid(row))
            self.assertTrue(row['parser_error_location_unavailable'])
            self.assertEqual(row['recoveries'],[])
            self.assertEqual(row['precheck']['truncated_files'],1)

    def test_forged_clean_and_unversioned_reports_are_rejected(self):
        row=json.loads((ROOT/'tests/acceptance/evidence/hidden-error-probe-2026-10-06.json').read_text())['reports'][0]
        for key,value in [('schema_version','0.1.0'),('parser_error_location_unavailable',False),('grammar_qualified',True),('delivery_decision','allow'),('next_action','run_or_configure_applicable_native_lint_before_delivery')]:
            bad=copy.deepcopy(row);bad[key]=value
            self.assertFalse(validator('grammar-probe-v0.5.schema.json').is_valid(bad))
        bad=copy.deepcopy(row);bad['precheck']['truncated_files']=0
        self.assertFalse(validator('grammar-probe-v0.5.schema.json').is_valid(bad))

if __name__=='__main__': unittest.main()
