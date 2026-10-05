"""JavaScript 隔离原生开发差分验收，不授予项目 lint 或 grammar 资格。"""
import copy
import hashlib
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class JavascriptDifferential(unittest.TestCase):
    def report(self):
        return json.loads((ROOT/'tests/acceptance/evidence/javascript-native-grammar-differential-2026-10-05.json').read_text())

    def test_actual_source_mode_denominator_and_raw_misses(self):
        report=self.report()
        validator('native-grammar-differential-v0.4.schema.json').validate(report)
        self.assertFalse(validator('native-grammar-differential-v0.3.schema.json').is_valid(report))
        corpus=(ROOT/'tests/acceptance/evidence/javascript-native-grammar-input-2026-10-05.json').read_bytes()
        validator('grammar-regression-corpus-v0.2.schema.json').validate(json.loads(corpus))
        self.assertEqual(report['corpus_sha256'],hashlib.sha256(corpus).hexdigest())
        sources={row['id']:row for row in json.loads(corpus)['cases'] if row['language']=='javascript'}
        self.assertEqual(report['sample_count'],18)
        self.assertEqual(report['language_count'],32)
        self.assertEqual(report['grammar_qualified_count'],0)
        self.assertFalse(report['independent_holdout'])
        self.assertFalse(report['native_adapter_reused'])
        js=next(row for row in report['languages'] if row['language']=='javascript')
        self.assertEqual((js['tp'],js['fp'],js['fn'],js['tn']),(5,0,2,11))
        self.assertEqual(js['compared_count'],18)
        self.assertEqual(js['combined_candidate'],{'tp':5,'fp':0,'fn':2,'tn':11,'compared_count':18,'unknown_count':0})
        for row in report['cases']:
            self.assertEqual(row['native']['input_type'],'module')
            self.assertTrue(row['native_identity_current'])
            self.assertFalse(row['fixture_native_disagreement'])
            self.assertEqual(row['source_sha256'],hashlib.sha256(sources[row['id']]['source'].encode()).hexdigest())
            self.assertEqual(row['structural_observations'],[])
            self.assertEqual(row['comparison'],row['combined_candidate_comparison'])
            self.assertEqual(row['native']['diagnostics'],[] if sources[row['id']]['expected_valid'] else [{'line':row['native']['diagnostics'][0]['line'],'rule_id':'javascript.syntax'}])
        for name in ['javascript-module_return','javascript-duplicate_binding']:
            row=next(row for row in report['cases'] if row['id']==name)
            self.assertEqual(row['comparison'],'false_negative')
        for name in ['javascript-no_execution','javascript-no_import_resolution']:
            row=next(row for row in report['cases'] if row['id']==name)
            self.assertEqual(row['native_classification'],'valid')

    def test_wrong_mode_native_identity_or_classification_is_rejected(self):
        schema=validator('native-grammar-differential-v0.4.schema.json')
        for edit in ['mode','language','rule','line','wrong_class','version','qualification','unexpected_field','stale_program','stale_native']:
            report=self.report();row=next(row for row in report['cases'] if row['native']['diagnostics'])
            if edit=='mode':row['native']['input_type']='commonjs'
            elif edit=='language':row['language']='python'
            elif edit=='rule':row['native']['diagnostics'][0]['rule_id']='eslint/no-unused-vars'
            elif edit=='line':row['native']['diagnostics'][0]['line']=0
            elif edit=='wrong_class':row['native_classification']='valid'
            elif edit=='version':row['native']['version']='v24.17.0'
            elif edit=='qualification':report['grammar_qualified_count']=1
            elif edit=='unexpected_field':row['native']['diagnostics'][0]['column']=1
            elif edit=='stale_program':report['program_stable']=False
            else:row['native_identity_current']=False
            self.assertFalse(schema.is_valid(report),edit)

    def test_incomplete_native_is_valid_only_when_both_comparisons_are_unknown(self):
        report=self.report();row=report['cases'][0]
        row['native'].update(status='incomplete',reason='javascript_syntax_report_invalid',diagnostics=[])
        row.update(native_classification='unknown',fixture_native_disagreement=None,comparison='unknown',combined_candidate_comparison='unknown')
        schema=validator('native-grammar-differential-v0.4.schema.json')
        schema.validate(report)
        for field in ['comparison','combined_candidate_comparison']:
            changed=copy.deepcopy(report);changed['cases'][0][field]='true_negative'
            self.assertFalse(schema.is_valid(changed))

if __name__=='__main__':unittest.main()
