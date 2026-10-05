"""开发期 Zig 原生任务反馈协议验收，不参与产品运行时。"""
import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from zig_aggregate_feedback_schema import BINARY, validator

class NativeFirst(unittest.TestCase):
    def test_actual_cross_entry_reports(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            source = root / 'app.zig'
            source.write_text('pub fn main( void {\n')
            tool = root / 'zig'
            tool.write_text('#!/bin/sh\nif [ "$1" = version ]; then printf "0.16.0\\n"; exit 0; fi\nIFS= read -r line || :\ncase "$line" in *"const Empty"*) exit 0;; esac\nprintf "<stdin>:1:13: error: failure\\n" >&2\nexit 1\n')
            tool.chmod(0o700)
            env = dict(os.environ, PATH=str(root))
            def run(*args, request=None):
                result = subprocess.run([str(BINARY), *map(str,args), '--format=json'], env=env, input=None if request is None else json.dumps(request).encode(), capture_output=True)
                self.assertIn(result.returncode, [0,3], result.stderr.decode())
                return json.loads(result.stdout)
            run('init',root,'--apply')
            scan = run('check','zig',root)
            validator('check-feedback-v0.47.schema.json').validate(scan)
            validator('check-feedback-v0.45.schema.json').validate(run('check','python',root))
            task_id = scan['native_results']['zig_lint']['files'][0]['task_id']
            self.assertIsInstance(task_id,str)
            native_reports = [json.loads(p.read_text()) for p in (root/'.codeguard/reports').glob('*.json')]
            observation = next(r for r in native_reports if r.get('schema_version') == '0.6.0')
            validator('syntax-confirmation-observation-v0.6.schema.json').validate(observation)
            for args,schema in [(('next',root),'repair-brief-preview-v0.12.schema.json'),(('task','show',task_id,root),'task-show-preview-v0.2.schema.json'),(('lint','zig',source,'--zig-tool',tool),'zig-lint-feedback-v0.3.schema.json')]:
                validator(schema).validate(run(*args))
            def hook(event):
                return run('hook','execute',root,'--timeout=30s',request={'schema_version':'1.0.0','report_type':'hook_trigger_request','input':{'event':event,'changed_paths':['app.zig'] if event=='file_changed' else [],'task_id':task_id,'write_outcome':'confirmed','host_claims_blocking':False}})
            validator('hook-execution-feedback-v0.16.schema.json').validate(hook('file_changed'))
            validator('hook-execution-feedback-v0.15.schema.json').validate(hook('repair_ready'))
            for clean in [False,True]:
                if clean: source.write_text('const Empty = struct {};\n')
                verify = run('task','verify',task_id,root,'--zig-tool',tool)
                validator('task-verification-preview-v0.19.schema.json').validate(verify)
                self.assertTrue(verify['event_persisted'])
                self.assertEqual(verify['observation'],'candidate_absent_unverified_policy' if clean else 'still_blocked')
                altered = copy.deepcopy(verify)
                altered['delivery_decision'] = 'allow'
                self.assertFalse(validator('task-verification-preview-v0.19.schema.json').is_valid(altered))
            fact=json.loads((root/f'.codeguard/findings/{task_id}/finding.json').read_text())
            self.assertEqual(fact['state'],'open')

if __name__ == '__main__':
    unittest.main()
