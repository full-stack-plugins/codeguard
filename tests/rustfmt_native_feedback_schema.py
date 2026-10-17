"""开发期 Rust 原生差分报告正反例，不参与产品检查运行时。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator


class RustfmtFeedback(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.report = json.loads((ROOT / 'tests/acceptance/evidence/rustfmt-native-differential-2026-10-06.json').read_text())

    def test_current_capture_has_scoped_parser_authority_and_consistent_counts(self):
        report = self.report
        validator('native-grammar-differential-v0.8.schema.json').validate(report)
        self.assertEqual(report['language_count'], 32)
        self.assertEqual(report['grammar_qualified_count'], 0)
        self.assertFalse(report['independent_holdout'])
        rows = report['cases']
        self.assertEqual(len(rows), 16)
        inventory = next(row for row in report['languages'] if row['language'] == 'rust')
        for key, comparison in [('tp', 'true_positive'), ('tn', 'true_negative'), ('fp', 'false_positive'), ('fn', 'false_negative')]:
            self.assertEqual(inventory[key], sum(row['comparison'] == comparison for row in rows))
        for row in rows:
            self.assertEqual(row['native']['observation_kind'], 'formatter_parser')
            self.assertEqual(row['native']['edition'], '2024')
            self.assertFalse(row['fixture_native_disagreement'])
        self.assertNotIn('HOST_SECRET', json.dumps(report))

    def test_false_authority_versions_editions_and_tool_diagnostics_are_rejected(self):
        check = validator('native-grammar-differential-v0.8.schema.json')
        for key, value in [('delivery_decision', 'allow'), ('grammar_qualified_count', 1), ('schema_version', '0.7.0')]:
            bad = copy.deepcopy(self.report)
            bad[key] = value
            self.assertFalse(check.is_valid(bad), key)
        for key, value in [('edition', '2015'), ('observation_kind', 'lint'), ('tool_sha256', 'bad'), ('version', 'rustfmt 1.8.0-stable')]:
            bad = copy.deepcopy(self.report)
            bad['cases'][0]['native'][key] = value
            self.assertFalse(check.is_valid(bad), key)
        bad = copy.deepcopy(self.report)
        bad['cases'][0]['native_attempted'] = False
        self.assertFalse(check.is_valid(bad))
        bad = copy.deepcopy(self.report)
        bad['cases'][0]['native_classification'] = 'invalid'
        self.assertFalse(check.is_valid(bad))


if __name__ == '__main__':
    unittest.main()
