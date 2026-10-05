"""开发期验证：读取真实 CLI 输出；不参与 Rust 检查运行时。"""
import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from jsonschema import Draft202012Validator
from referencing import Registry, Resource

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / 'target/debug/codeguard'
SCHEMA = json.loads((ROOT / 'schemas/check-feedback-v0.45.schema.json').read_text())
SCHEMAS = {p.name: json.loads(p.read_text()) for p in (ROOT / 'schemas').glob('*.schema.json')}
REGISTRY = Registry().with_resources([(n, Resource.from_contents(s)) for n, s in SCHEMAS.items()] + [(s['$id'], Resource.from_contents(s)) for s in SCHEMAS.values() if '$id' in s])
VALIDATOR = Draft202012Validator(SCHEMA, registry=REGISTRY)

class SelectionProtocol(unittest.TestCase):
    def check(self, root, language):
        env = dict(os.environ, PATH='')
        env.pop('CODEGUARD_TIMEOUT', None)
        env.pop('CODEGUARD_JOBS', None)
        result = subprocess.run([str(BINARY), 'check', language, str(root), '--format=json'], env=env, capture_output=True, check=False)
        self.assertEqual(result.returncode, 3, result.stderr.decode())
        return json.loads(result.stdout)

    def test_every_new_canonical_selection_has_valid_actual_feedback(self):
        with tempfile.TemporaryDirectory() as root:
            for language in SCHEMA['properties']['selection']['enum']:
                with self.subTest(language=language):
                    report = self.check(root, language)
                    VALIDATOR.validate(report)
                    self.assertEqual(report['selection'], language)

    def test_wrong_language_native_result_and_allow_are_rejected(self):
        with tempfile.TemporaryDirectory() as root:
            Path(root, 'main.py').write_text('import os\n')
            report = self.check(root, 'python')
            VALIDATOR.validate(report)
            altered = copy.deepcopy(report)
            altered['delivery_decision'] = 'allow'
            self.assertFalse(VALIDATOR.is_valid(altered))
            altered = copy.deepcopy(report)
            altered['native_results']['java_p3c'] = {}
            self.assertFalse(VALIDATOR.is_valid(altered))
            altered = copy.deepcopy(report)
            altered['category_candidates'][0]['language'] = 'rust'
            self.assertFalse(VALIDATOR.is_valid(altered))

    def test_selected_wasm_report_retains_candidate_authority(self):
        with tempfile.TemporaryDirectory() as root:
            Path(root, 'main.c').write_text('int main(void) { return 0; }\n')
            Path(root, 'main.py').write_text('def broken(:\n')
            report = self.check(root, 'c')
            VALIDATOR.validate(report)
            self.assertEqual(report['syntax_candidates']['source_file_count'], 1)
            for observation in report['syntax_candidates']['observations']:
                self.assertEqual(observation['language'], 'c')
                self.assertFalse(observation['grammar_qualified'])

    def test_persisted_candidate_next_keeps_the_next_view_shape(self):
        with tempfile.TemporaryDirectory() as root:
            Path(root, 'main.c').write_text('int main( {\n')
            env = dict(os.environ, PATH='')
            initialized = subprocess.run([str(BINARY), 'init', root, '--apply', '--format=json'], env=env, capture_output=True)
            self.assertEqual(initialized.returncode, 3)
            report = self.check(root, 'c')
            VALIDATOR.validate(report)
            self.assertEqual(report['next']['repair_brief']['task_id'], report['syntax_tasks']['tasks'][0]['task_id'])

    def test_actual_native_ruff_report_has_the_scoped_protocol(self):
        path = ROOT / 'tests/acceptance/evidence/python-language-selection-2026-10-05.json'
        report = json.loads(path.read_text())
        VALIDATOR.validate(report)
        self.assertEqual(report['selection'], 'python')
        self.assertIsNone(report['native_results']['java_p3c'])
        self.assertTrue(any(finding['rule_id'] == 'F401' for file in report['native_results']['python_lint']['files'] for finding in file['findings']))

    def test_new_schema_definitions_are_valid_and_old_ids_are_not_rewritten(self):
        Draft202012Validator.check_schema(SCHEMA)
        Draft202012Validator.check_schema(json.loads((ROOT / 'schemas/check-aborted-v0.14.schema.json').read_text()))
        prior = json.loads((ROOT / 'schemas/check-feedback-v0.44.schema.json').read_text())
        self.assertEqual(prior['properties']['schema_version']['const'], '0.44.0')
        self.assertEqual(prior['properties']['selection']['enum'], ['all', 'java'])

if __name__ == '__main__':
    unittest.main()
