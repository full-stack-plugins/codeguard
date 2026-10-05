"""Go整文件结构候选的实际公开协议、历史隔离和伪造反例；开发验收专用。"""
import copy
import json
import os
import pathlib
import subprocess
import tempfile
import unittest
from zig_aggregate_feedback_schema import ROOT, validator


class GoPackageStructure(unittest.TestCase):
    def command(self, *args, request=None):
        process = subprocess.run(
            [str(ROOT / 'target/debug/codeguard'), *args],
            env={**os.environ, 'PATH': ''},
            input=None if request is None else json.dumps(request).encode(),
            capture_output=True, timeout=130, check=False,
        )
        self.assertEqual(process.returncode, 3, process.stderr.decode())
        return json.loads(process.stdout)

    def test_actual_public_versions_and_forged_imports(self):
        with tempfile.TemporaryDirectory(prefix='cg-go-package-schema-') as directory:
            root = pathlib.Path(directory).resolve()
            (root / 'go.mod').write_text('module example.test/sample\n\ngo 1.23\n')
            source = root / 'main.go'
            source.write_text('// package fake\nfunc f() {}\n')
            self.command('init', str(root), '--apply', '--format=json')
            probe = self.command('grammar', 'probe', 'go', str(source), '--format=json')
            probe_schema = validator('grammar-probe-v0.3.schema.json')
            probe_schema.validate(probe)
            self.assertFalse(validator('grammar-probe-v0.2.schema.json').is_valid(probe))
            self.assertEqual(probe['recoveries'], [])
            self.assertEqual(probe['precheck']['suspected_recoveries'], 0)
            for field, value in [('source_scope', 'fragment'), ('language', 'python'), ('grammar_qualified', True)]:
                changed = copy.deepcopy(probe); changed[field] = value
                self.assertFalse(probe_schema.is_valid(changed), field)
            for field, value in [('rule_sha256', '0'*64), ('start_byte', 1), ('parent_syntax_kind', 'module')]:
                changed = copy.deepcopy(probe); changed['structural_observations'][0][field] = value
                self.assertFalse(probe_schema.is_valid(changed), field)
            report = self.command('check', 'go', str(root), '--format=json')
            validator('repair-brief-preview-v0.13.schema.json').validate(report['next'])
            self.assertFalse(validator('repair-brief-preview-v0.1.schema.json').is_valid(report['next']))
            aggregate_schema = validator('check-feedback-v0.49.schema.json')
            aggregate_schema.validate(report)
            self.assertFalse(validator('check-feedback-v0.48.schema.json').is_valid(report))
            task = next(row['task_id'] for row in report['syntax_tasks']['tasks'] if row['path'] == 'main.go')
            hook = self.command('hook', 'execute', str(root), '--timeout=30s', '--format=json', request={
                'schema_version': '1.0.0', 'report_type': 'hook_trigger_request',
                'input': {'event': 'file_changed', 'changed_paths': ['main.go'], 'task_id': None,
                          'write_outcome': 'confirmed', 'host_claims_blocking': False},
            })
            validator('hook-execution-feedback-v0.18.schema.json').validate(hook)
            self.assertFalse(validator('hook-execution-feedback-v0.17.schema.json').is_valid(hook))
            self.assertEqual(hook['local_feedback']['next_action'], 'require_native_lint_confirmation')
            self.assertEqual(hook['local_feedback']['syntax_tasks']['tasks'][0]['task_id'], task)
            confirmation = next(json.loads(path.read_text()) for path in (root / '.codeguard/reports').glob('*.json')
                                if json.loads(path.read_text()).get('schema_version') == '0.8.0'
                                and json.loads(path.read_text()).get('report_type') == 'syntax_confirmation_observation')
            schema = validator('syntax-confirmation-observation-v0.8.schema.json')
            schema.validate(confirmation)
            self.assertFalse(validator('syntax-confirmation-observation-v0.7.schema.json').is_valid(confirmation))
            for index, (field, value) in enumerate([('rule_sha256', '0'*64), ('start_byte', 1)]):
                changed = copy.deepcopy(confirmation)
                changed['run_id'] = f'syntax-confirm-999-{index+1}'
                changed['observations'][0]['structural_observations'][0][field] = value
                self.assertFalse(schema.is_valid(changed))
                (root / '.codeguard/reports' / f"{changed['run_id']}.json").write_text(json.dumps(changed))
            changed = copy.deepcopy(confirmation)
            changed['run_id'] = 'syntax-confirm-999-3'
            changed['observations'][0]['structural_observation_count'] = 2
            changed['observations'][0]['structural_observations'] *= 2
            self.assertFalse(schema.is_valid(changed))
            (root / '.codeguard/reports' / f"{changed['run_id']}.json").write_text(json.dumps(changed))
            sync = self.command('work', 'sync', str(root), '--format=json')
            self.assertEqual(sync['failed_reports'], 3)
            self.assertEqual(sync['new_blockers'], 0)
            changed = copy.deepcopy(report)
            row = next(row for row in changed['syntax_candidates']['observations'] if row['path'] == 'main.go')
            row['language'] = 'python'
            self.assertFalse(aggregate_schema.is_valid(changed))
            changed = copy.deepcopy(report['next'])
            changed['repair_brief']['recheck_argv'][2] = 'python'
            self.assertFalse(validator('repair-brief-preview-v0.13.schema.json').is_valid(changed))

    def test_new_native_report_preserves_old_raw_evidence(self):
        evidence = ROOT / 'tests/acceptance/evidence'
        old = json.loads((evidence / 'go-native-grammar-differential-2026-10-05.json').read_bytes())
        new = json.loads((evidence / 'go-native-grammar-package-differential-2026-10-05.json').read_bytes())
        validator('native-grammar-differential-v0.6.schema.json').validate(old)
        schema = validator('native-grammar-differential-v0.7.schema.json')
        schema.validate(new)
        self.assertFalse(validator('native-grammar-differential-v0.6.schema.json').is_valid(new))
        self.assertEqual(new['corpus_sha256'], old['corpus_sha256'])
        rows = {row['id']: row for row in new['cases']}
        for old_row in old['cases']:
            row = rows[old_row['id']]
            for field in ['source_sha256', 'fixture_label', 'wasm_classification', 'wasm_recovery_count', 'comparison', 'native_classification']:
                self.assertEqual(row[field], old_row[field], (old_row['id'], field))
        for name in ['go-missing_package_statement', 'go-missing_package_function']:
            row = rows[name]
            self.assertEqual(row['comparison'], 'false_negative')
            self.assertEqual(row['combined_candidate_comparison'], 'true_positive')
            self.assertEqual(len(row['structural_observations']), 1)
        self.assertEqual(rows['go-logical_line_positions_unresolved']['combined_candidate_comparison'], 'unknown')
        for field, value in [('rule_sha256', '0'*64), ('end_byte', 1), ('rule_id', 'codeguard.python.required_suite')]:
            changed = copy.deepcopy(new)
            row = next(row for row in changed['cases'] if row['id'] == 'go-missing_package_statement')
            row['structural_observations'][0][field] = value
            self.assertFalse(schema.is_valid(changed), field)

    def test_eight_language_replay_preserves_all_other_observations(self):
        evidence = ROOT / 'tests/acceptance/evidence'
        old = json.loads((evidence / 'native-differential-eight-language-go-2026-10-05.json').read_bytes())
        new = json.loads((evidence / 'native-differential-eight-language-go-package-2026-10-05.json').read_bytes())
        validator('native-grammar-differential-v0.7.schema.json').validate(new)
        self.assertEqual((new['sample_count'], new['language_count'], new['selected_language_count']), (152, 32, 8))
        self.assertEqual(new['grammar_qualified_count'], 0)
        self.assertFalse(new['independent_holdout'])
        old_rows = {row['id']: row for row in old['cases']}
        for row in new['cases']:
            before = old_rows[row['id']]
            for field in ['source_sha256', 'origin', 'fixture_label', 'fixture_expected_valid', 'wasm_classification', 'wasm_recovery_count', 'native_classification', 'comparison']:
                self.assertEqual(row[field], before[field], (row['id'], field))
            if row['language'] != 'go':
                self.assertEqual(row['structural_observations'], before['structural_observations'])
                self.assertEqual(row['combined_candidate_comparison'], before['combined_candidate_comparison'])
        counts = {name: sum(row['combined_candidate_comparison'] == name for row in new['cases'])
                  for name in ['true_positive', 'false_positive', 'false_negative', 'true_negative', 'unknown']}
        self.assertEqual(counts, {'true_positive': 43, 'false_positive': 0, 'false_negative': 12, 'true_negative': 91, 'unknown': 6})


if __name__ == '__main__':
    unittest.main()
