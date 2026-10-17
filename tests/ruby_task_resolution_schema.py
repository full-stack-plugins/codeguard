"""Ruby 限定宿主协议的开发验收；不参与产品检测运行时。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator


class RubyResolutionSchema(unittest.TestCase):
    def rows(self, kind):
        return json.loads((ROOT / f'tests/acceptance/evidence/ruby-task-resolution-{kind}-2026-10-06.json').read_text())

    def test_real_original_tool_closure_and_recurrence_for_both_origins(self):
        rows = self.rows('real')
        self.assertEqual(len(rows), 4)
        self.assertEqual(sorted(r['receipt']['state'] for r in rows), ['open', 'open', 'resolved', 'resolved'])
        self.assertEqual(sum(r['policy']['grammar_sha256'] is None for r in rows), 2)
        for row in rows:
            self.validate(row)
            self.assertEqual(row['evidence']['original_native']['status'], 'diagnostics_observed')
            for key in ['original_native', 'current_native']:
                self.assertEqual(row['evidence'][key]['version'], 'ruby 2.6.10p210')
                self.assertEqual(row['evidence'][key]['tool_sha256'], row['policy']['tool_sha256'])
                for diagnostic in row['evidence'][key]['diagnostics']:
                    self.assertNotIn('column', diagnostic)

    def validate(self, row):
        for kind, version in [('policy', '1.9'), ('evidence', '0.10'), ('receipt', '0.1')]:
            validator(f'task-resolution-{kind}-v{version}.schema.json').validate(row[kind])
        self.assertEqual(row['receipt']['delivery_decision'], 'not_evaluated')
        self.assertEqual(row['policy']['identity'], row['evidence']['identity'])
        self.assertEqual(row['policy']['grammar_sha256'], row['evidence']['grammar_sha256'])

    def test_controlled_counterexample_and_version_mutation_are_not_closure(self):
        rows = self.rows('controlled')
        for row in rows:
            self.validate(row)
        outcomes = {r['receipt']['outcome'] for r in rows}
        self.assertTrue({'code_fixed', 'false_positive_review_required', 'inputs_stale'} <= outcomes)
        for row in rows:
            if row['receipt']['outcome'] != 'code_fixed':
                self.assertNotEqual(row['receipt']['state'], 'resolved')

    def test_foreign_versions_fabricated_columns_and_false_clean_are_rejected(self):
        row = self.rows('real')[0]
        for kind, version in [('policy', '1.9'), ('evidence', '0.10')]:
            for field, value in [('approved', True), ('schema_version', '9.0.0')]:
                doc = copy.deepcopy(row[kind]); doc[field] = value
                self.assertFalse(validator(f'task-resolution-{kind}-v{version}.schema.json').is_valid(doc))
        for field, value in [('line', 0), ('column', 1), ('rule_id', 'erlang.syntax.error')]:
            doc = copy.deepcopy(row['evidence']); doc['original_native']['diagnostics'][0][field] = value
            self.assertFalse(validator('task-resolution-evidence-v0.10.schema.json').is_valid(doc))
        doc = copy.deepcopy(row['evidence']); doc['outcome'] = 'code_fixed'; doc['current_native'] = doc['original_native']
        self.assertFalse(validator('task-resolution-evidence-v0.10.schema.json').is_valid(doc))
        doc = copy.deepcopy(row['policy']); doc['native_version'] = 'ruby 3.4.0'
        self.assertFalse(validator('task-resolution-policy-v1.9.schema.json').is_valid(doc))


if __name__ == '__main__':
    unittest.main()
