"""C/C++实际原生文档报告的独立协议验收；不参与产品运行或授予资格。"""
import copy
import hashlib
import json
import subprocess
import unittest

from zig_aggregate_feedback_schema import ROOT, SCHEMAS, validator
from jsonschema import Draft202012Validator


class CFamilyComments(unittest.TestCase):
    def evidence(self, filename):
        return json.loads((ROOT / 'tests/acceptance/evidence' / filename).read_text())

    def test_actual_native_cases_keep_rules_scope_and_missing_comment_gap(self):
        for filename in ['c-family-comments-native.json', 'c-family-comments-native-wasm.json']:
            evidence = self.evidence(filename)
            self.assertEqual(evidence['evidence_kind'], 'local_native_development_regression')
            self.assertFalse(evidence['independent_holdout'])
            self.assertEqual(evidence['qualification'], 'not_granted')
            self.assertEqual(evidence['test_source_sha256'], hashlib.sha256(
                (ROOT / 'crates/codeguard-cli/tests/c_family_comments_cli.rs').read_bytes()).hexdigest())
            self.assertEqual(len(evidence['cases']), 16)
            for case in evidence['cases']:
                report = case['report']
                validator('c-family-comments-feedback-v0.1.schema.json').validate(report)
                self.assertEqual(len(report['documentation_findings']), case['expected_documentation_diagnostics'])
                self.assertEqual(report['verification_command'][3], report['path'])
                self.assertEqual(report['verification_command'][2], report['language'])
                self.assertEqual(report['verification_command'][7], report['documentation_configuration']['standard'])
                native = report['native']['diagnostics']
                self.assertEqual(len(native), len(report['documentation_findings']))
                for finding in report['documentation_findings']:
                    self.assertTrue(any(all(finding[key] == item[key] for key in ['rule_id', 'line', 'column_byte', 'level']) for item in native))
                if case['case_id'].endswith('-missing'):
                    self.assertEqual(report['documentation_findings'], [])
                    self.assertEqual(report['detailed_contract_qualification'], 'not_granted')
                    self.assertEqual(report['documentation_configuration']['rule_coverage'], 'limited')

    def test_qualification_context_and_incomplete_result_forgery_is_rejected(self):
        original = self.evidence('c-family-comments-native.json')['cases'][0]['report']
        variants = []
        for key, value in [('coverage_proven', True), ('delivery_decision', 'allow'),
                           ('detailed_contract_qualification', 'passed'), ('next', {'task_id': 'CG-forged'}),
                           ('schema_version', '0.2.0'), ('authority', 'trusted')]:
            report = copy.deepcopy(original)
            report[key] = value
            variants.append(report)
        report = copy.deepcopy(original)
        report['documentation_findings'][0]['rule_id'] = 'clang.warn_doc_new_unadapted_rule'
        variants.append(report)
        report = copy.deepcopy(original)
        report['documentation_configuration']['standard'] = 'c++17'
        variants.append(report)
        report = copy.deepcopy(original)
        report['native']['status'] = 'incomplete'
        report['native']['reason'] = 'clang_tool_changed'
        variants.append(report)
        report = copy.deepcopy(original)
        report['native']['diagnostics'][0]['level'] = 'error'
        variants.append(report)
        report = copy.deepcopy(original)
        report['approved'] = True
        variants.append(report)
        report = copy.deepcopy(original)
        report['command_status'] = 'cancelled'
        variants.append(report)
        for index, report in enumerate(variants):
            self.assertFalse(validator('c-family-comments-feedback-v0.1.schema.json').is_valid(report), index)
        self.assertFalse(validator('syntax-lint-feedback-v0.2.schema.json').is_valid(original))

    def test_all_schemas_are_valid_and_historical_schemas_are_unchanged(self):
        for schema in SCHEMAS.values():
            Draft202012Validator.check_schema(schema)
        names = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', 'HEAD', 'schemas'], cwd=ROOT, text=True).splitlines()
        checked = 0
        for name in names:
            if not name.endswith('.schema.json'):
                continue
            expected = subprocess.check_output(['git', 'show', 'HEAD:' + name], cwd=ROOT)
            self.assertEqual((ROOT / name).read_bytes(), expected, name)
            checked += 1
        self.assertGreaterEqual(checked, 477)


if __name__ == '__main__':
    unittest.main()
