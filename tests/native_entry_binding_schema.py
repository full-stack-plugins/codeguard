"""六语言真实入口复核后的固定语料对照；保留未知及原始缺陷。"""
import hashlib
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

EVIDENCE = ROOT / 'tests/acceptance/evidence'


class NativeEntryBindingEvidence(unittest.TestCase):
    def test_actual_six_language_reports_keep_scope_counts_and_unknowns(self):
        for suffix, count in [('base-', 98), ('', 114)]:
            report = json.loads((EVIDENCE / f'native-differential-six-language-entry-binding-{suffix}2026-10-05.json').read_text())
            validator('native-grammar-differential-v0.4.schema.json').validate(report)
            self.assertEqual(report['sample_count'], count)
            self.assertEqual(report['selected_language_count'], 6)
            self.assertEqual(report['language_count'], 32)
            self.assertEqual(report['grammar_qualified_count'], 0)
            self.assertFalse(report['native_adapter_reused'])
            self.assertFalse(report['independent_holdout'])
            self.assertEqual(report['delivery_decision'], 'not_evaluated')
            self.assertTrue(report['program_stable'])
            rows = {r['language']: r for r in report['languages'] if r['native_selected']}
            self.assertEqual(set(rows), {'zig', 'erlang', 'swift', 'kotlin', 'python', 'javascript'})
            expected = {'zig': (4, 0, 0, 8, 0, 0), 'erlang': (6, 0, 10, 20, 2, 0),
                        'swift': (4, 0, 0, 9, 0, 1), 'kotlin': (4, 0, 0, 8, 0, 2),
                        'javascript': (5, 0, 2, 11, 0, 0),
                        'python': (1, 0, 0, 1, 0, 0) if count == 98 else (6, 0, 2, 10, 0, 0)}
            for language, values in expected.items():
                self.assertEqual(tuple(rows[language][k] for k in ('tp', 'fp', 'fn', 'tn', 'native_unknown_count', 'wasm_unknown_count')), values)
            self.assertEqual(sum(r['comparison'] == 'unknown' for r in report['cases']), 5)
            self.assertTrue(all(r['native_identity_current'] for r in report['cases']))
            self.assertTrue(all(r['fixture_native_disagreement'] is not True for r in report['cases']))

    def test_expansion_adds_existing_python_cases_without_relabelling_base(self):
        base_input = json.loads((EVIDENCE / 'javascript-native-grammar-input-cancellation-2026-10-05.json').read_bytes())
        expanded_bytes = (EVIDENCE / 'native-differential-six-language-entry-input-2026-10-05.json').read_bytes()
        expanded = json.loads(expanded_bytes)
        validator('grammar-regression-corpus-v0.2.schema.json').validate(expanded)
        expected = base_input['cases'] + json.loads((ROOT / 'tests/fixtures/python_syntax_regression.json').read_bytes())['cases']
        self.assertEqual(expanded['cases'], expected)
        report = json.loads((EVIDENCE / 'native-differential-six-language-entry-binding-2026-10-05.json').read_text())
        self.assertEqual(report['corpus_sha256'], hashlib.sha256(expanded_bytes).hexdigest())
        by_id = {r['id']: r for r in expanded['cases']}
        for row in report['cases']:
            source = by_id[row['id']]
            self.assertEqual(row['source_sha256'], hashlib.sha256(source['source'].encode()).hexdigest())
            self.assertEqual(row['fixture_expected_valid'], source['expected_valid'])
        base_report = json.loads((EVIDENCE / 'native-differential-six-language-entry-binding-base-2026-10-05.json').read_text())
        rows = {r['id']: r for r in report['cases']}
        for old in base_report['cases']:
            for field in ('source_sha256', 'grammar_sha256', 'comparison', 'combined_candidate_comparison'):
                self.assertEqual(rows[old['id']][field], old[field])
        self.assertEqual(report['program_sha256'], base_report['program_sha256'])


if __name__ == '__main__':
    unittest.main()
