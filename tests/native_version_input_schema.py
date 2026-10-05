"""版本动作输入/输出修复后的真实Zig/Kotlin开发对照证据。"""
import hashlib
import json
import unittest
from zig_aggregate_feedback_schema import ROOT, validator


class NativeVersionInputEvidence(unittest.TestCase):
    def test_actual_report_preserves_selected_samples_and_unknowns(self):
        directory = ROOT / 'tests/acceptance/evidence'
        report = json.loads((directory / 'native-version-input-zig-kotlin-2026-10-05.json').read_text())
        validator('native-grammar-differential-v0.1.schema.json').validate(report)
        self.assertEqual(report['sample_count'], 26)
        self.assertEqual(report['selected_language_count'], 2)
        self.assertEqual(report['language_count'], 32)
        self.assertEqual(report['grammar_qualified_count'], 0)
        self.assertTrue(report['program_stable'])
        self.assertFalse(report['independent_holdout'])
        self.assertEqual(report['delivery_decision'], 'not_evaluated')
        selected = {r['language']: r for r in report['languages'] if r['native_selected']}
        self.assertEqual(set(selected), {'zig', 'kotlin'})
        for language in selected:
            self.assertEqual(tuple(selected[language][k] for k in ('tp', 'fp', 'fn', 'tn')), (4, 0, 0, 8))
            self.assertEqual(selected[language]['compared_count'], 12)
            self.assertEqual(selected[language]['native_unknown_count'], 0)
        self.assertEqual(selected['zig']['wasm_unknown_count'], 0)
        self.assertEqual(selected['kotlin']['wasm_unknown_count'], 2)
        self.assertTrue(all(r['native_identity_current'] for r in report['cases']))
        previous = json.loads((directory / 'native-differential-six-language-entry-binding-2026-10-05.json').read_text())
        old = {r['id']: r for r in previous['cases'] if r['language'] in selected}
        self.assertEqual(set(old), {r['id'] for r in report['cases']})
        for row in report['cases']:
            for key in ('source_sha256', 'grammar_sha256', 'native_classification', 'wasm_classification', 'comparison'):
                self.assertEqual(row[key], old[row['id']][key])
        corpus = (directory / 'native-differential-six-language-entry-input-2026-10-05.json').read_bytes()
        self.assertEqual(report['corpus_sha256'], hashlib.sha256(corpus).hexdigest())


if __name__ == '__main__':
    unittest.main()
