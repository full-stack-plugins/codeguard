"""Ruby 原生入口的真实输出与封闭协议验收；任务链路另行验收。"""
import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class RubyLintFeedback(unittest.TestCase):
    def test_live_fallback_and_explicit_failure(self):
        binary=os.environ.get('CODEGUARD_RUBY_BIN',str(ROOT/'target/debug/codeguard'))
        with tempfile.TemporaryDirectory(prefix='cg-ruby-feedback-') as directory:
            source=Path(directory)/'app.rb';source.write_text('def f(\n')
            for extra in [[],['--ruby-tool','/missing/ruby']]:
                out=subprocess.run([binary,'lint','ruby',str(source),'--format=json',*extra],env=dict(os.environ,PATH=''),capture_output=True,timeout=35)
                self.assertEqual(out.returncode,3,out.stderr.decode())
                data=json.loads(out.stdout);v=validator('ruby-lint-feedback-v0.1.schema.json');v.validate(data)
                for key,value in [('delivery_decision','allow'),('authority','trusted'),('coverage_proven',True),('project_version_compatibility','confirmed'),('approved',True)]:
                    bad=copy.deepcopy(data);bad[key]=value;self.assertFalse(v.is_valid(bad))
    def test_actual_ruby_captures_and_line_only_contract(self):
        directory=os.environ.get('CODEGUARD_RUBY_ENTRY_EVIDENCE_DIR')
        if not directory:self.skipTest('select actual existing Ruby captures')
        v=validator('ruby-lint-feedback-v0.1.schema.json')
        for name,status in [('broken','diagnostics_observed'),('clean','completed')]:
            data=json.loads((Path(directory)/f'{name}.json').read_text());v.validate(data)
            self.assertEqual(data['native']['status'],status)
            if name=='broken':
                bad=copy.deepcopy(data);bad['native']['diagnostics'][0]['column']=1
                self.assertFalse(v.is_valid(bad))
                bad=copy.deepcopy(data);bad['native']['diagnostics'][0]['rule_id']='rubocop.Style'
                self.assertFalse(v.is_valid(bad))
    def test_live_help_adds_ruby_without_rewriting_old_help(self):
        binary=os.environ.get('CODEGUARD_RUBY_BIN',str(ROOT/'target/debug/codeguard'))
        out=subprocess.run([binary,'help','lint','--format=json'],capture_output=True,timeout=10)
        self.assertEqual(out.returncode,0)
        data=json.loads(out.stdout);validator('command-help-v0.4.schema.json').validate(data)
        self.assertIn('ruby',data['commands'][0]['languages'])
        for version in ['0.1','0.2','0.3']:self.assertFalse(validator(f'command-help-v{version}.schema.json').is_valid(data))

    def test_archived_workflow_and_native_origin(self):
        prefix=ROOT/'tests/acceptance/evidence'
        for name,schema in [('fallback','ruby-lint-feedback-v0.3.schema.json'),('native','ruby-lint-feedback-v0.3.schema.json'),('next','repair-brief-preview-v0.15.schema.json'),('next-after-repair','repair-brief-preview-v0.15.schema.json'),('still-blocked','task-verification-preview-v0.23.schema.json'),('verify','task-verification-preview-v0.23.schema.json'),('native-origin','syntax-confirmation-observation-v0.9.schema.json')]:
            validator(schema).validate(json.loads((prefix/f'ruby-task-workflow-2026-10-05-{name}.json').read_text()))
    def test_actual_task_workflow_captures(self):
        directory=os.environ.get('CODEGUARD_RUBY_EVIDENCE_DIR')
        if not directory:self.skipTest('select actual original-tool workflow captures')
        directory=Path(directory)
        for name,schema in [('fallback','ruby-lint-feedback-v0.3.schema.json'),('native','ruby-lint-feedback-v0.3.schema.json'),('next','repair-brief-preview-v0.15.schema.json'),('next-after-repair','repair-brief-preview-v0.15.schema.json'),('still-blocked','task-verification-preview-v0.23.schema.json'),('verify','task-verification-preview-v0.23.schema.json')]:
            data=json.loads((directory/f'{name}.json').read_text());validator(schema).validate(data)
        verify=json.loads((directory/'verify.json').read_text())
        self.assertEqual(verify['observation'],'candidate_absent_unverified_policy')
        for key,value in [('delivery_decision','allow'),('authority','trusted'),('approved',True)]:
            bad=copy.deepcopy(verify);bad[key]=value;self.assertFalse(validator('task-verification-preview-v0.23.schema.json').is_valid(bad))
        next_report=json.loads((directory/'next.json').read_text())
        bad=copy.deepcopy(next_report);bad['repair_brief']['native_diagnostic_positions'][0]['column']=1
        self.assertFalse(validator('repair-brief-preview-v0.15.schema.json').is_valid(bad))

if __name__=='__main__':unittest.main()
