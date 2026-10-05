"""开发验收：独立Rust lint真实CLI反馈，不赋予发布或原生资格。"""
import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class RustLintFeedback(unittest.TestCase):
    def test_archived_actual_reports(self):
        for name in ['native','candidate','zero-candidates']:
            data=json.loads((ROOT/f'tests/acceptance/evidence/rust-standalone-lint-2026-10-05-{name}.json').read_text())
            validator('rust-lint-feedback-v0.1.schema.json').validate(data)
        verify=json.loads((ROOT/'tests/acceptance/evidence/rust-standalone-lint-2026-10-05-verify.json').read_text())
        validator('task-verification-preview-v0.6.schema.json').validate(verify)
        help_report=json.loads((ROOT/'tests/acceptance/evidence/rust-standalone-lint-2026-10-05-help.json').read_text())
        validator('command-help-v0.2.schema.json').validate(help_report)
    def test_actual_missing_native_and_invalid_explicit_tool(self):
        binary=os.environ.get('CODEGUARD_RUST_LINT_BIN',str(ROOT/'target/debug/codeguard'))
        with tempfile.TemporaryDirectory(prefix='cg-rust-lint-schema-') as directory:
            root=Path(directory);(root/'src').mkdir();(root/'src/lib.rs').write_text('pub fn broken( {\n')
            (root/'Cargo.toml').write_text("[package]\nname='schema-sample'\nversion='0.1.0'\nedition='2021'\n")
            (root/'Cargo.lock').write_text('version = 4\n')
            for extra in [[],['--cargo-tool','/missing/cargo']]:
                env=dict(os.environ,PATH='');env.pop('CODEGUARD_TIMEOUT',None)
                out=subprocess.run([binary,'lint','rust',str(root),'--format=json',*extra],env=env,capture_output=True,timeout=30)
                self.assertEqual(out.returncode,3,out.stderr.decode());data=json.loads(out.stdout)
                validator('rust-lint-feedback-v0.1.schema.json').validate(data)
                for field,value in [('schema_version','2.0.0'),('delivery_decision','allow'),('authority','trusted'),('coverage_proven',True),('approved',True)]:
                    wrong=copy.deepcopy(data);wrong[field]=value
                    self.assertFalse(validator('rust-lint-feedback-v0.1.schema.json').is_valid(wrong))
                wrong=copy.deepcopy(data);wrong['native_tool_requirement']='available'
                self.assertFalse(validator('rust-lint-feedback-v0.1.schema.json').is_valid(wrong))
    def test_actual_native_and_original_recheck_capture(self):
        directory=os.environ.get('CODEGUARD_RUST_LINT_EVIDENCE_DIR')
        if not directory:self.skipTest('select actual native captures')
        native=json.loads((Path(directory)/'native.json').read_text());validator('rust-lint-feedback-v0.1.schema.json').validate(native)
        self.assertTrue(native['native_report']['local_scan_complete'])
        recheck=json.loads((Path(directory)/'verify.json').read_text());validator('task-verification-preview-v0.6.schema.json').validate(recheck)
        self.assertEqual(recheck['observation'],'candidate_absent_unverified_policy')
    def test_current_help_reports_rust_and_keeps_old_protocol(self):
        out=subprocess.run([os.environ.get('CODEGUARD_RUST_LINT_BIN',str(ROOT/'target/debug/codeguard')),'help','lint','--format=json'],capture_output=True,timeout=10)
        data=json.loads(out.stdout);validator('command-help-v0.3.schema.json').validate(data)
        self.assertIn('rust',data['commands'][0]['languages'])
        self.assertFalse(validator('command-help-v0.1.schema.json').is_valid(data))

if __name__=='__main__':unittest.main()
