"""开发验收：首次 Kotlin 原生扫描实际输出及协议拒绝边界。"""
import copy
import json
import unittest
from pathlib import Path
from jsonschema import Draft202012Validator
from referencing import Registry, Resource
ROOT = Path(__file__).resolve().parents[1]

class KotlinNativeFirstSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schemas = {p.name: json.loads(p.read_text()) for p in (ROOT / 'schemas').glob('*.schema.json')}
        cls.registry = Registry().with_resources([(n, Resource.from_contents(s)) for n, s in cls.schemas.items()] + [(s['$id'], Resource.from_contents(s)) for s in cls.schemas.values() if '$id' in s])
        evidence = json.loads((ROOT / 'tests/acceptance/evidence/kotlin-native-first-2026-10-04.json').read_text())
        cls.reports = {n: r['report'] for n, r in evidence['reports'].items()}

    def validator(self, name):
        return Draft202012Validator(self.schemas[name], registry=self.registry)

    def test_actual_reports(self):
        for n in ['bad', 'repeat', 'clean', 'mixed', 'missing']:
            with self.subTest(n=n): self.validator('check-feedback-v0.42.schema.json').validate(self.reports[n])
        self.validator('hook-execution-feedback-v0.11.schema.json').validate(self.reports['hook'])
        self.validator('syntax-confirmation-observation-v0.4.schema.json').validate(self.reports['origin'])
        self.validator('task-verification-preview-v0.17.schema.json').validate(self.reports['verify'])
        for n in ['next_bad', 'next_clean', 'next_mixed']:
            self.validator('repair-brief-preview-v0.10.schema.json').validate(self.reports[n])

    def test_actual_counts_and_stable_task_identity(self):
        task_ids = []
        for name in ['bad', 'repeat', 'clean', 'mixed', 'missing']:
            report = self.reports[name]
            scan = report['native_results']['kotlin_lint']
            self.assertEqual(scan['source_file_count'], len(scan['files']) + scan['unobserved_count'])
            self.assertEqual(report['execution_budget']['native_task_count'], 1)
            self.assertLessEqual(report['execution_budget']['started_native_task_count'], 1)
            task_ids.append(scan['files'][0]['task_id'])
        self.assertEqual(len(set(task_ids)), 1)
        self.assertEqual(self.reports['hook']['local_feedback']['kotlin_lint']['files'][0]['task_id'], task_ids[0])

    def test_old_consumers_reject_new_versions(self):
        for n, schema in [('bad', 'check-feedback-v0.41.schema.json'), ('hook', 'hook-execution-feedback-v0.10.schema.json'), ('verify', 'task-verification-preview-v0.16.schema.json')]:
            self.assertFalse(self.validator(schema).is_valid(self.reports[n]), n)

    def test_scan_cannot_forge_coverage_currentness_or_columns(self):
        original = self.reports['bad']['native_results']['kotlin_lint']
        validator = self.validator('kotlin-compile-scan-v0.2.schema.json')
        for key, value in [('coverage_proven', True), ('delivery_decision', 'allow'), ('authority', 'trusted')]:
            r = copy.deepcopy(original); r[key] = value
            self.assertFalse(validator.is_valid(r), key)
        r = copy.deepcopy(original); r['files'][0]['current'] = False
        self.assertFalse(validator.is_valid(r))
        r = copy.deepcopy(original); r['files'][0]['native']['diagnostics'][0]['column_byte'] = 0
        self.assertFalse(validator.is_valid(r))

    def test_native_origin_has_no_fabricated_grammar(self):
        r = copy.deepcopy(self.reports['verify'])
        r['native_scan']['original_report']['grammar_sha256'] = 'a' * 64
        self.assertFalse(self.validator('task-verification-preview-v0.17.schema.json').is_valid(r))
        r = copy.deepcopy(self.reports['origin']); r['language'] = 'erlang'
        self.assertFalse(self.validator('syntax-confirmation-observation-v0.4.schema.json').is_valid(r))

if __name__ == '__main__': unittest.main()
