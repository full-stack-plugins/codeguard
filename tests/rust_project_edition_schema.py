"""开发期项目 edition 解析观察协议验收，不参与产品运行时。"""
import copy
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class RustProjectEditionSchema(unittest.TestCase):
    def test_real_tool_reports_bind_matching_editions_and_keep_scope(self):
        reports = json.loads((ROOT / 'tests/acceptance/evidence/rust-project-edition-2026-10-06.json').read_text())
        self.assertEqual(len(reports), 3)
        for report, edition, status in zip(reports, ['2015', '2021', '2024'], ['diagnostics_observed', 'completed', 'completed']):
            validator('rust-project-syntax-observation-v0.1.schema.json').validate(report)
            self.assertEqual(report['edition_context']['edition'], edition)
            self.assertEqual(report['native']['edition'], edition)
            self.assertEqual(report['status'], status)
            self.assertFalse(report['coverage_proven'])
            self.assertEqual(report['delivery_decision'], 'not_evaluated')
            self.assertNotIn('pub async', json.dumps(report))

    def test_forged_scope_or_incompatible_edition_is_rejected(self):
        reports = json.loads((ROOT / 'tests/acceptance/evidence/rust-project-edition-2026-10-06.json').read_text())
        v = validator('rust-project-syntax-observation-v0.1.schema.json')
        for key, value in [('coverage_proven', True), ('delivery_decision', 'allow'), ('native', None), ('edition_context', None), ('source_sha256', None), ('status', 'diagnostics_observed'), ('status', 'incomplete')]:
            bad = copy.deepcopy(reports[1]); bad[key] = value
            self.assertFalse(v.is_valid(bad), key)
        bad = copy.deepcopy(reports[1]); bad['native']['edition'] = '2024'
        self.assertFalse(v.is_valid(bad))
        bad = copy.deepcopy(reports[1]); bad['edition_context']['manifest_ref'] = '/private/project/Cargo.toml'
        self.assertFalse(v.is_valid(bad))
        for report in reports[:2]:
            self.assertFalse(validator('rustfmt-syntax-observation-v0.1.schema.json').is_valid(report['native']))
        self.assertTrue(validator('rustfmt-syntax-observation-v0.1.schema.json').is_valid(reports[2]['native']))

if __name__ == '__main__':
    unittest.main()
