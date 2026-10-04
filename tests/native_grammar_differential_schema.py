"""开发期原生差分协议校验，不提供 grammar 资格或项目门禁授权。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class NativeDifferential(unittest.TestCase):
    def report(self):
        return json.loads((ROOT/'tests/acceptance/evidence/native-grammar-differential-2026-10-05.json').read_text())
    def test_actual_scope_counts_and_protocol(self):
        report=self.report()
        validator('native-grammar-differential-v0.1.schema.json').validate(report)
        self.assertEqual(report['language_count'],32)
        self.assertEqual(report['selected_language_count'],4)
        self.assertEqual(report['sample_count'],78)
        self.assertFalse(report['independent_holdout'])
        self.assertTrue(report['native_adapter_reused'])
        mixed=next(c for c in report['cases'] if c['id']=='kotlin-missing_expression')
        self.assertEqual(mixed['native']['status'],'incomplete')
        self.assertEqual(mixed['native_classification'],'invalid')
        self.assertEqual(mixed['comparison'],'true_positive')
        for language in report['languages']:
            if not language['native_selected']:continue
            rows=[r for r in report['cases'] if r['language']==language['language']]
            self.assertEqual(language['sample_count'],len(rows))
            self.assertEqual(language['compared_count'],sum(r['comparison']!='unknown' for r in rows))
            self.assertEqual(language['native_unknown_count'],sum(r['native_classification']=='unknown' for r in rows))
            self.assertEqual(language['wasm_unknown_count'],sum(r['wasm_classification']=='unknown' for r in rows))
            for key,comparison in [('tp','true_positive'),('fp','false_positive'),('fn','false_negative'),('tn','true_negative')]:
                self.assertEqual(language[key],sum(r['comparison']==comparison for r in rows))
    def test_report_cannot_be_promoted_or_stale_as_valid(self):
        for field,value in [('independent_holdout',True),('grammar_qualified_count',4),('delivery_decision','allow')]:
            changed=self.report();changed[field]=value
            self.assertFalse(validator('native-grammar-differential-v0.1.schema.json').is_valid(changed))
        changed=self.report();changed['program_stable']=False
        self.assertFalse(validator('native-grammar-differential-v0.1.schema.json').is_valid(changed))
        changed=self.report();row=next(r for r in changed['cases'] if r['native_classification']!='unknown');row['native_identity_current']=False
        self.assertFalse(validator('native-grammar-differential-v0.1.schema.json').is_valid(changed))

if __name__=='__main__':unittest.main()
