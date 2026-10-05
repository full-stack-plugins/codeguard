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

    def test_python_actual_syntax_only_protocol_and_rejections(self):
        report=json.loads((ROOT/'tests/acceptance/evidence/python-native-grammar-differential-2026-10-05.json').read_text())
        schema=validator('native-grammar-differential-v0.2.schema.json')
        schema.validate(report)
        self.assertFalse(validator('native-grammar-differential-v0.1.schema.json').is_valid(report))
        self.assertEqual(report['sample_count'],18)
        self.assertEqual(report['language_count'],32)
        for row in report['cases']:
            self.assertEqual(row['language'],'python')
            self.assertEqual(row['native']['target_version'],'py312')
            self.assertFalse(row['fixture_native_disagreement'])
        for case_id in ['python-unused_import','python-unresolved_name']:
            row=next(r for r in report['cases'] if r['id']==case_id)
            self.assertEqual(row['native_classification'],'valid')
        row=next(r for r in report['cases'] if r['id']=='python-noqa_cannot_hide')
        self.assertEqual(row['native_classification'],'invalid')
        changed=copy.deepcopy(report);changed['cases'][0]['language']='zig'
        self.assertFalse(schema.is_valid(changed))

        changed=copy.deepcopy(report);changed['cases'][0]['native']['status']='incomplete'
        self.assertFalse(schema.is_valid(changed))
        changed=copy.deepcopy(report);row=next(r for r in changed['cases'] if r['native']['diagnostics']);row['native']['diagnostics'][0]['rule_id']='F401'
        self.assertFalse(schema.is_valid(changed))

    def test_python_structure_candidate_keeps_raw_metrics_and_rule_identity(self):
        report=json.loads((ROOT/'tests/acceptance/evidence/python-native-structure-differential-2026-10-05.json').read_text())
        schema=validator('native-grammar-differential-v0.3.schema.json')
        schema.validate(report)
        self.assertFalse(validator('native-grammar-differential-v0.2.schema.json').is_valid(report))
        python=next(row for row in report['languages'] if row['language']=='python')
        self.assertEqual((python['tp'],python['fp'],python['fn'],python['tn']),(6,0,2,10))
        combined=python['combined_candidate']
        self.assertEqual((combined['tp'],combined['fp'],combined['fn'],combined['tn']),(8,0,0,10))
        historical=json.loads((ROOT/'tests/acceptance/evidence/python-native-grammar-differential-current-manifest-2026-10-05.json').read_text())
        old_rows={row['id']:row for row in historical['cases']}
        self.assertEqual({row['id'] for row in report['cases']},set(old_rows))
        for row in report['cases']:
            for key in ['source_sha256','grammar_sha256','native_classification','wasm_classification','comparison','fixture_expected_valid','fixture_label','cohort','origin']:
                self.assertEqual(row[key],old_rows[row['id']][key],(row['id'],key))
        self.assertEqual(combined['compared_count']+combined['unknown_count'],python['sample_count'])
        for key,label in [('tp','true_positive'),('fp','false_positive'),('fn','false_negative'),('tn','true_negative'),('unknown_count','unknown')]:
            self.assertEqual(combined[key],sum(row['combined_candidate_comparison']==label for row in report['cases']))
        for case_id in ['python-empty_body','python-bad_indent']:
            row=next(row for row in report['cases'] if row['id']==case_id)
            self.assertEqual(row['comparison'],'false_negative')
            self.assertEqual(row['wasm_recovery_count'],0)
            self.assertEqual(row['combined_candidate_comparison'],'true_positive')
            self.assertEqual(row['structural_observations'][0]['rule_id'],'codeguard.python.required_suite')
        for edit in ['promote','erase_structure','forge_rule','stale_native','stale_program','unknown_parser','false_comparison','extra_field']:
            changed=copy.deepcopy(report)
            row=next(row for row in changed['cases'] if row['id']=='python-empty_body')
            if edit=='promote':changed['combined_candidate_authority']='confirmed_violation'
            elif edit=='erase_structure':row['structural_observations']=[]
            elif edit=='forge_rule':row['structural_observations'][0]['rule_sha256']='a'*64
            elif edit=='stale_native':row['native_identity_current']=False
            elif edit=='stale_program':changed['program_stable']=False
            elif edit=='unknown_parser':row['wasm_classification']='unknown'
            elif edit=='false_comparison':row['combined_candidate_comparison']='true_negative'
            else:row['structural_observations'][0]['kind']='ERROR'
            self.assertFalse(schema.is_valid(changed),edit)

    def test_structure_report_identity_retraction_and_unknown_are_schema_valid(self):
        report=json.loads((ROOT/'tests/acceptance/evidence/python-native-structure-differential-2026-10-05.json').read_text())
        schema=validator('native-grammar-differential-v0.3.schema.json')
        # 撤回程序身份必须清除全部候选，不允许沿用结构观察。
        report['program_stable']=False
        for row in report['cases']:
            row.update(wasm_classification='unknown',wasm_recovery_count=None,wasm_reason='grammar_evaluation_program_changed',comparison='unknown',structural_observations=None,combined_candidate_classification='unknown',combined_candidate_comparison='unknown')
        python=next(row for row in report['languages'] if row['language']=='python')
        python.update(wasm_unknown_count=18,compared_count=0,tp=0,fp=0,fn=0,tn=0)
        python['combined_candidate'].update(compared_count=0,unknown_count=18,tp=0,fp=0,fn=0,tn=0)
        schema.validate(report)

if __name__=='__main__':unittest.main()
