"""Ruby 隔离原生对照及版本化协议，保留原分母和未验收边界。"""
import hashlib
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

EVIDENCE = ROOT / 'tests/acceptance/evidence'

class RubyNativeDifferential(unittest.TestCase):
    def report(self):
        return json.loads((EVIDENCE / 'ruby-native-grammar-differential-2026-10-05.json').read_bytes())

    def test_actual_ruby_evidence_is_bound_to_frozen_sources_and_native_version(self):
        report = self.report()
        validator('native-grammar-differential-v0.5.schema.json').validate(report)
        self.assertFalse(validator('native-grammar-differential-v0.4.schema.json').is_valid(report))
        data = (EVIDENCE / 'ruby-native-grammar-input-2026-10-05.json').read_bytes()
        corpus = json.loads(data)
        validator('grammar-regression-corpus-v0.2.schema.json').validate(corpus)
        self.assertEqual(report['corpus_sha256'], hashlib.sha256(data).hexdigest())
        self.assertEqual((report['sample_count'], report['language_count'], report['selected_language_count']), (18,32,1))
        self.assertEqual(report['grammar_qualified_count'], 0)
        self.assertFalse(report['independent_holdout'])
        self.assertFalse(report['native_adapter_reused'])
        self.assertTrue(report['program_stable'])
        ruby = next(r for r in report['languages'] if r['language']=='ruby')
        self.assertEqual(tuple(ruby[k] for k in ('tp','fp','fn','tn','native_unknown_count','wasm_unknown_count')), (5,0,0,13,0,0))
        by_id = {r['id']:r for r in corpus['cases']}
        for row in report['cases']:
            self.assertTrue(row['native_identity_current'])
            self.assertFalse(row['fixture_native_disagreement'])
            self.assertEqual(row['native']['version'], 'ruby 2.6.10p210')
            self.assertEqual(row['source_sha256'], hashlib.sha256(by_id[row['id']]['source'].encode()).hexdigest())
            self.assertEqual(row['combined_candidate_comparison'], row['comparison'])
            validator('ruby-native-syntax.schema.json').validate(row['native'])
        for name in ['ruby-no_execution','ruby-no_require_resolution','ruby-no_shebang_require','ruby-utf8_encoding']:
            self.assertEqual(next(r for r in report['cases'] if r['id']==name)['native_classification'], 'valid')

    def test_wrong_version_rule_classification_or_authority_is_rejected(self):
        schema = validator('native-grammar-differential-v0.5.schema.json')
        for edit in ['version','rule','wrong_class','identity','program','qualification','extra_column','language','zero_line']:
            report = self.report()
            row = next(r for r in report['cases'] if r['native']['diagnostics'])
            if edit=='version': row['native']['version']='ruby 3.0.0'
            elif edit=='rule': row['native']['diagnostics'][0]['rule_id']='rubocop/Layout'
            elif edit=='wrong_class': row['native_classification']='valid'
            elif edit=='identity': row['native_identity_current']=False
            elif edit=='program': report['program_stable']=False
            elif edit=='qualification': report['grammar_qualified_count']=1
            elif edit=='extra_column': row['native']['diagnostics'][0]['column']=1
            elif edit=='language': row['language']='javascript'
            else: row['native']['diagnostics'][0]['line']=0
            self.assertFalse(schema.is_valid(report), edit)

    def test_incomplete_native_keeps_both_comparisons_unknown(self):
        report=self.report(); row=report['cases'][0]
        row['native'].update(status='incomplete',reason='ruby_syntax_report_invalid',diagnostics=[])
        row.update(native_classification='unknown',comparison='unknown',combined_candidate_comparison='unknown',fixture_native_disagreement=None)
        schema=validator('native-grammar-differential-v0.5.schema.json')
        schema.validate(report)
        row['combined_candidate_comparison']='true_negative'
        self.assertFalse(schema.is_valid(report))

    def test_actual_seven_language_report_preserves_old_six_results_and_missing_coverage(self):
        report=json.loads((EVIDENCE/'native-differential-seven-language-ruby-2026-10-05.json').read_bytes())
        validator('native-grammar-differential-v0.5.schema.json').validate(report)
        self.assertEqual((report['sample_count'],report['selected_language_count'],report['language_count']),(132,7,32))
        self.assertEqual(report['grammar_qualified_count'],0)
        self.assertFalse(report['independent_holdout'])
        self.assertTrue(report['program_stable'])
        selected=[r for r in report['languages'] if r['native_selected']]
        self.assertTrue(all(r['tool_stable'] for r in selected))
        self.assertEqual(tuple(sum(r[k] for r in selected) for k in ('tp','fp','fn','tn')),(34,0,14,79))
        self.assertEqual(sum(r['comparison']=='unknown' for r in report['cases']),5)
        self.assertEqual(sum(not r['native_selected'] for r in report['languages']),25)
        data=(EVIDENCE/'native-differential-seven-language-ruby-input-2026-10-05.json').read_bytes()
        self.assertEqual(report['corpus_sha256'],hashlib.sha256(data).hexdigest())
        rows={r['id']:r for r in report['cases']}
        old=json.loads((EVIDENCE/'native-differential-six-language-entry-binding-2026-10-05.json').read_bytes())
        for row in old['cases']:
            for key in ['source_sha256','grammar_sha256','fixture_expected_valid','fixture_label','native_classification','wasm_classification','comparison','combined_candidate_classification','combined_candidate_comparison']:
                self.assertEqual(rows[row['id']][key],row[key],(row['id'],key))
        input_rows={r['id']:r for r in json.loads(data)['cases']}
        for row in report['cases']:
            self.assertEqual(row['source_sha256'],hashlib.sha256(input_rows[row['id']]['source'].encode()).hexdigest())
        ruby=next(r for r in selected if r['language']=='ruby')
        self.assertEqual(tuple(ruby[k] for k in ('tp','fp','fn','tn')),(5,0,0,13))

    def test_seven_language_input_retains_all_previous_sources_and_labels(self):
        old=json.loads((EVIDENCE/'native-differential-six-language-entry-input-2026-10-05.json').read_bytes())
        data=(EVIDENCE/'native-differential-seven-language-ruby-input-2026-10-05.json').read_bytes()
        new=json.loads(data)
        validator('grammar-regression-corpus-v0.2.schema.json').validate(new)
        self.assertEqual(new['cases'][:len(old['cases'])],old['cases'])
        self.assertEqual(len(new['cases']),len(old['cases'])+4)

if __name__=='__main__':
    unittest.main()
