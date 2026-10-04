"""开发期实际反馈协议验收；不参与产品检查运行时。"""
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
SCHEMAS = {p.name: json.loads(p.read_text()) for p in (ROOT / 'schemas').glob('*.schema.json')}
REGISTRY = Registry().with_resources([(n, Resource.from_contents(s)) for n, s in SCHEMAS.items()] + [(s['$id'], Resource.from_contents(s)) for s in SCHEMAS.values() if '$id' in s])

def validator(name):
    return Draft202012Validator(SCHEMAS[name], registry=REGISTRY)

class ZigFeedback(unittest.TestCase):
    def test_actual_real_selected_feedback(self):
        evidence = json.loads((ROOT / 'tests/acceptance/evidence/zig-aggregate-2026-10-05.json').read_text())
        for key, status in [('broken', 'diagnostics_observed'), ('clean', 'completed')]:
            report = evidence[key]
            validator('check-feedback-v0.46.schema.json').validate(report)
            self.assertFalse(validator('check-feedback-v0.45.schema.json').is_valid(report))
            self.assertEqual(report['native_results']['zig_lint']['files'][0]['native']['status'], status)
            altered = copy.deepcopy(report)
            altered['delivery_decision'] = 'allow'
            self.assertFalse(validator('check-feedback-v0.46.schema.json').is_valid(altered))
        broken = evidence['broken']
        altered = copy.deepcopy(broken)
        altered['native_results']['zig_lint']['files'][0]['native']['status'] = 'completed'
        self.assertFalse(validator('check-feedback-v0.46.schema.json').is_valid(altered))
        altered = copy.deepcopy(broken)
        altered['native_results']['zig_lint']['files'][0]['current'] = False
        self.assertFalse(validator('check-feedback-v0.46.schema.json').is_valid(altered))

    def test_actual_all_and_hook_missing_tool_protocol(self):
        with tempfile.TemporaryDirectory() as root:
            Path(root, 'app.zig').write_text('pub fn main( void {\n')
            env = dict(os.environ, PATH='')
            result = subprocess.run([str(BINARY), 'check', 'all', root, '--format=json'], env=env, capture_output=True)
            self.assertEqual(result.returncode, 3)
            report = json.loads(result.stdout)
            validator('check-feedback-v0.46.schema.json').validate(report)
            self.assertEqual(report['native_results']['zig_lint']['tool_selection']['source'], 'not_found')
            request = {'schema_version':'1.0.0', 'report_type':'hook_trigger_request', 'input':{'event':'file_changed', 'changed_paths':['app.zig'], 'write_outcome':'confirmed', 'host_claims_blocking':False}}
            result = subprocess.run([str(BINARY), 'hook', 'execute', root, '--format=json', '--timeout=30s'], env=env, input=json.dumps(request).encode(), capture_output=True)
            self.assertEqual(result.returncode, 3, result.stderr.decode())
            feedback = json.loads(result.stdout)
            validator('hook-execution-feedback-v0.14.schema.json').validate(feedback)
            self.assertFalse(validator('hook-execution-feedback-v0.13.schema.json').is_valid(feedback))

    def test_actual_selected_tool_failure_and_native_hook(self):
        with tempfile.TemporaryDirectory() as root:
            Path(root, 'app.zig').write_text('pub fn main( void {\n')
            tool = Path(root, 'zig')
            tool.write_text('#!/bin/sh\nif [ "$1" = version ]; then printf "0.15.0\\n"; exit 0; fi\nexit 1\n')
            tool.chmod(0o700)
            env = dict(os.environ, PATH=root)
            result = subprocess.run([str(BINARY), 'check', 'zig', root, '--format=json'], env=env, capture_output=True)
            report = json.loads(result.stdout)
            validator('check-feedback-v0.46.schema.json').validate(report)
            self.assertEqual(report['syntax_candidates']['observations'], [])
            request = {'schema_version':'1.0.0', 'report_type':'hook_trigger_request', 'input':{'event':'file_changed', 'changed_paths':['app.zig'], 'write_outcome':'confirmed', 'host_claims_blocking':False}}
            result = subprocess.run([str(BINARY), 'hook', 'execute', root, '--format=json', '--timeout=30s'], env=env, input=json.dumps(request).encode(), capture_output=True)
            report = json.loads(result.stdout)
            validator('hook-execution-feedback-v0.14.schema.json').validate(report)
            altered = copy.deepcopy(report)
            altered['local_feedback']['zig_lint']['local_parse_complete'] = True
            self.assertFalse(validator('hook-execution-feedback-v0.14.schema.json').is_valid(altered))

    def test_aborted_protocol_preserves_native_observation_and_refuses_allow(self):
        evidence = json.loads((ROOT / 'tests/acceptance/evidence/zig-aggregate-2026-10-05.json').read_text())['broken']
        schema = SCHEMAS['check-aborted-v0.15.schema.json']
        native = {key: evidence['native_results'].get(key) for key in schema['properties']['native_results']['properties']}
        packet = {'schema_version':'0.15.0', 'report_type':'check_aborted', 'operation':'check', 'selection':'all', 'command_status':'internal_error', 'exit_code':4, 'delivery_decision':'incomplete', 'authority':'local_unverified', 'reason':'native_task_internal_failure', 'failed_task_ids':['python.lint'], 'execution_tasks':[{'id':'python.lint','status':'internal_failure'},{'id':'zig.lint','status':'native_observed_unverified'}], 'discovery':evidence['discovery'], 'native_results':native, 'export':{'status':'not_requested','reason_code':None}}
        validator('check-aborted-v0.15.schema.json').validate(packet)
        self.assertFalse(validator('check-aborted.schema.json').is_valid(packet))
        packet['delivery_decision'] = 'allow'
        self.assertFalse(validator('check-aborted-v0.15.schema.json').is_valid(packet))

    def test_schemas_are_valid(self):
        for schema in SCHEMAS.values():
            Draft202012Validator.check_schema(schema)

if __name__ == '__main__':
    unittest.main()
