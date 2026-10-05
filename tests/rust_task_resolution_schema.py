"""限定Rust关闭协议开发验收；真实原生工具与测试宿主信任来源分开。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator


def artifacts(mode):
    return json.loads((ROOT / f'tests/acceptance/evidence/rust-task-resolution-{mode}-2026-10-06.json').read_text())


class RustResolutionSchema(unittest.TestCase):
    def test_actual_real_tool_resolution_and_recurrence(self):
        rows = artifacts('real')
        self.assertEqual(len(rows), 4)
        self.assertEqual([r['receipt']['state'] for r in rows], ['resolved', 'resolved', 'open', 'resolved'])
        self.assertEqual(rows[0]['receipt']['event_ref'], rows[1]['receipt']['event_ref'])
        for row in rows:
            self.validate_row(row)
            self.assertEqual(row['evidence']['original_native']['status'], 'diagnostics_observed')
            for key in ['original_native', 'current_native']:
                self.assertEqual(row['evidence'][key]['edition_context'], row['policy']['edition_context'])
                self.assertEqual(row['evidence'][key]['tool_sha256'], row['policy']['tool_sha256'])

    def validate_row(self, row):
        for kind, version in [('policy', '1.7'), ('evidence', '0.8'), ('receipt', '0.1')]:
            validator(f'task-resolution-{kind}-v{version}.schema.json').validate(row[kind])
        self.assertEqual(row['receipt']['delivery_decision'], 'not_evaluated')
        self.assertEqual(row['policy']['identity'], row['evidence']['identity'])
        self.assertIsNone(row['evidence']['grammar_sha256'])

    def test_controlled_mutation_and_counterexample_remain_unresolved(self):
        rows = artifacts('controlled')
        for row in rows:
            self.validate_row(row)
        outcomes = {r['receipt']['outcome'] for r in rows}
        self.assertTrue({'code_fixed', 'still_present', 'false_positive_review_required', 'inputs_stale'} <= outcomes)
        for row in rows:
            if row['receipt']['outcome'] in ['inputs_stale', 'false_positive_review_required']:
                self.assertNotEqual(row['receipt']['state'], 'resolved')

    def test_missing_context_foreign_protocol_and_forged_positions_are_rejected(self):
        row = artifacts('real')[0]
        for kind, version in [('policy', '1.7'), ('evidence', '0.8')]:
            doc = copy.deepcopy(row[kind]); del doc['edition_context']
            self.assertFalse(validator(f'task-resolution-{kind}-v{version}.schema.json').is_valid(doc))
            for field, value in [('grammar_sha256', '0' * 64), ('approved', True), ('schema_version', '9.0.0')]:
                doc = copy.deepcopy(row[kind]); doc[field] = value
                self.assertFalse(validator(f'task-resolution-{kind}-v{version}.schema.json').is_valid(doc))
        for version in ['1.0', '1.4', '1.6']:
            self.assertFalse(validator(f'task-resolution-policy-v{version}.schema.json').is_valid(row['policy']))
        for field, value in [('line', 0), ('column', 1), ('rule_id', 'go.syntax')]:
            doc = copy.deepcopy(row['evidence']); doc['original_native']['diagnostics'][0][field] = value
            self.assertFalse(validator('task-resolution-evidence-v0.8.schema.json').is_valid(doc))
        doc = copy.deepcopy(row['evidence']); doc['current_native']['status'] = 'diagnostics_observed'
        self.assertFalse(validator('task-resolution-evidence-v0.8.schema.json').is_valid(doc))


if __name__ == '__main__':
    unittest.main()
