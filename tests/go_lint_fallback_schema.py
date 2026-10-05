"""Go统一lint缺工具候选反馈的实际协议验收；仅开发测试。"""
import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class GoLintFallback(unittest.TestCase):
    def command(self, root, *args):
        output = subprocess.run([str(ROOT/'target/debug/codeguard'), *args, str(root), '--format', 'json'], env={**os.environ,'PATH':''},capture_output=True,timeout=90)
        self.assertEqual(output.returncode, 3, output.stderr.decode())
        return json.loads(output.stdout)
    def test_actual_missing_tool_candidates_and_recommendations(self):
        schema = validator('go-lint-fallback-feedback-v0.7.schema.json')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            (root/'go.mod').write_text('module example.test/sample\n\ngo 1.23\n')
            for source, expected in [('func f() {}\n','required'),('package main\nfunc f() {}\n','recommended')]:
                (root/'main.go').write_text(source)
                report = self.command(root, 'lint','go')
                schema.validate(report)
                self.assertEqual(report['native_tool_requirement'],expected)
                self.assertFalse(validator('go-lint-local-feedback.schema.json').is_valid(report))
                changed = copy.deepcopy(report);changed['delivery_decision']='allow'
                self.assertFalse(schema.is_valid(changed))
                changed = copy.deepcopy(report);changed['syntax_candidates']['observations'][0]['language']='python'
                self.assertFalse(schema.is_valid(changed))
                changed = copy.deepcopy(report);changed['native_report']['native_status']='completed'
                self.assertFalse(schema.is_valid(changed))
                if expected=='required':
                    changed = copy.deepcopy(report);changed['native_tool_requirement']='recommended'
                    self.assertFalse(schema.is_valid(changed))
                else:
                    changed = copy.deepcopy(report);changed['syntax_candidates']['skipped_count']=1
                    self.assertFalse(schema.is_valid(changed))
    @unittest.skipUnless(os.environ.get('CODEGUARD_DEFAULT_GO_BINARY'), 'requires separately saved binary without wasm-precheck')
    def test_default_binary_reports_missing_wasm_without_fake_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory).resolve();(root/'main.go').write_text('package main\nfunc f() {}\n')
            output=subprocess.run([os.environ['CODEGUARD_DEFAULT_GO_BINARY'],'lint','go',str(root),'--format','json'],env={**os.environ,'PATH':''},capture_output=True,timeout=60)
            self.assertEqual(output.returncode,3)
            report=json.loads(output.stdout)
            validator('go-lint-fallback-feedback-v0.7.schema.json').validate(report)
            self.assertEqual(report['syntax_candidates']['reason'],'wasm_feature_not_built')
            self.assertEqual(report['preliminary_result'],'incomplete')
            self.assertEqual(report['native_tool_requirement'],'required')

    def test_initialized_stable_task_and_read_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory).resolve();(root/'go.mod').write_text('module example.test/sample\n\ngo 1.23\n')
            (root/'main.go').write_text('func f() {}\n')
            self.command(root,'init','--apply')
            first=self.command(root,'lint','go');second=self.command(root,'lint','go')
            validator('go-lint-fallback-feedback-v0.7.schema.json').validate(first)
            self.assertEqual(first['syntax_tasks']['tasks'][0]['task_id'],second['syntax_tasks']['tasks'][0]['task_id'])
            (root/'main.go').write_bytes(b'\xff')
            failed=self.command(root,'lint','go')
            validator('go-lint-fallback-feedback-v0.7.schema.json').validate(failed)
            self.assertEqual(failed['preliminary_result'],'incomplete')
            self.assertEqual(failed['native_tool_requirement'],'required')
if __name__=='__main__':unittest.main()
