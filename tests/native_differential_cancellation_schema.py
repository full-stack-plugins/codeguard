"""共享取消后的真实回归证据；旧报告不能被覆盖或提升资格。"""
import hashlib
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator


class NativeCancellationEvidence(unittest.TestCase):
    def test_actual_reports_keep_same_sources_raw_misses_and_authority(self):
        for language, old_name, new_name, schema, expected in [
            ('python', 'python-native-structure-differential-node-extension-2026-10-05.json',
             'python-native-structure-differential-cancellation-2026-10-05.json',
             'native-grammar-differential-v0.3.schema.json', (6, 0, 2, 10)),
            ('javascript', 'javascript-native-grammar-differential-2026-10-05.json',
             'javascript-native-grammar-differential-cancellation-2026-10-05.json',
             'native-grammar-differential-v0.4.schema.json', (5, 0, 2, 11)),
        ]:
            old = json.loads((ROOT / 'tests/acceptance/evidence' / old_name).read_text())
            new = json.loads((ROOT / 'tests/acceptance/evidence' / new_name).read_text())
            validator(schema).validate(new)
            self.assertEqual(new['sample_count'], 18)
            self.assertEqual(new['language_count'], 32)
            self.assertEqual(new['grammar_qualified_count'], 0)
            self.assertFalse(new['independent_holdout'])
            self.assertEqual(new['delivery_decision'], 'not_evaluated')
            row = next(r for r in new['languages'] if r['language'] == language)
            self.assertEqual(tuple(row[k] for k in ('tp', 'fp', 'fn', 'tn')), expected)
            fields = ('id', 'source_sha256', 'grammar_sha256', 'fixture_expected_valid',
                      'comparison', 'combined_candidate_comparison')
            self.assertEqual([[r[k] for k in fields] for r in old['cases']],
                             [[r[k] for k in fields] for r in new['cases']])
            self.assertTrue(new['program_stable'])
            self.assertTrue(all(r['native_identity_current'] for r in new['cases']))

    def test_previous_report_bytes_and_javascript_input_are_retained(self):
        for name, expected in [
            ('javascript-native-grammar-differential-2026-10-05.json',
             '3967e723912a067e8eb6fc18f0653c488670b9d2c637edc73b13f90cd933e785'),
            ('python-native-structure-differential-node-extension-2026-10-05.json',
             '17c9098e38ed0863fc53cffe8e356f45791e7f96376a2a9713725a81e5ef2587'),
        ]:
            self.assertEqual(hashlib.sha256((ROOT / 'tests/acceptance/evidence' / name).read_bytes()).hexdigest(), expected)
        directory = ROOT / 'tests/acceptance/evidence'
        original = (directory / 'javascript-native-grammar-input-2026-10-05.json').read_bytes()
        current = (directory / 'javascript-native-grammar-input-cancellation-2026-10-05.json').read_bytes()
        self.assertEqual(original, current)
        validator('grammar-regression-corpus-v0.2.schema.json').validate(json.loads(current))
        report = json.loads((directory / 'javascript-native-grammar-differential-cancellation-2026-10-05.json').read_text())
        self.assertEqual(report['corpus_sha256'], hashlib.sha256(current).hexdigest())


if __name__ == '__main__':
    unittest.main()
