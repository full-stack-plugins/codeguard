"""开发期实际Python单文件确认复检协议验收；不进入产品运行时。"""
import copy
import json
import os
import pathlib
import subprocess
import tempfile
import unittest
from zig_aggregate_feedback_schema import ROOT, validator

class PythonConfirmationRecheck(unittest.TestCase):
    def command(self, *args, empty_path=False, expected=3):
        env = dict(os.environ)
        if empty_path:
            env['PATH'] = ''
        result = subprocess.run([str(ROOT/'target/debug/codeguard'), *args], env=env, capture_output=True, timeout=130, check=False)
        self.assertEqual(result.returncode, expected, result.stderr.decode())
        self.assertEqual(result.stderr, b'')
        return json.loads(result.stdout)

    def test_actual_single_file_report_and_original_receipt_rejection(self):
        tool = os.environ['CODEGUARD_RUFF_BIN']
        with tempfile.TemporaryDirectory(prefix='cg-python-confirmation-recheck-') as directory:
            root = pathlib.Path(directory).resolve()
            (root/'ruff.toml').write_text("[lint]\nselect = ['F401']\n")
            (root/'broken.py').write_text('def run():\n')
            (root/'unrelated.py').write_text('import os\n')
            self.command('init', str(root), '--apply', '--format=json')
            first = self.command('lint', 'python', str(root), '--file', 'broken.py', '--format=json', empty_path=True)
            task = first['setup']['task_id']
            original = next(json.loads(p.read_text()) for p in (root/'.codeguard/reports').glob('*.json') if json.loads(p.read_text())['report_type']=='python_syntax_confirmation_observation')
            (root/'broken.py').write_text('def run():\n    pass\n')
            args = ('task', 'verify', task, str(root), '--ruff-tool', tool, '--format=json')
            result = self.command(*args)
            validator('task-verification-preview-v0.21.schema.json').validate(result)
            self.assertFalse(validator('task-verification-preview-v0.20.schema.json').is_valid(result))
            schema = validator('python-lint-feedback-v0.19.schema.json')
            scan = result['native_scan']
            schema.validate(scan)
            self.assertFalse(validator('python-lint-feedback-v0.18.schema.json').is_valid(scan))
            self.assertTrue(result['event_persisted'])
            self.assertEqual([f['path'] for f in scan['files']], ['broken.py'])
            self.assertEqual(scan['task_binding']['original_report']['source_sha256'], original['source_sha256'])
            self.assertNotEqual(scan['task_binding']['source_sha256'], original['source_sha256'])
            self.assertFalse(validator('task-verification-preview-v0.9.schema.json').is_valid(result))
            self.assertEqual(self.command('task', 'show', task, str(root), '--format=json', expected=0)['task']['task_id'], task)
            (root/'broken.py').write_text('value = 1\n')
            stale = self.command('task','show',task,str(root),'--format=json',expected=0)['task']
            self.assertEqual(stale['verification_invalidated_reason'], 'source_input_changed_or_unavailable')
            (root/'broken.py').write_text('def run():\n    pass\n')
            (root/'ruff.toml').write_text("[lint]\nselect = ['F821']\n")
            stale = self.command('task','show',task,str(root),'--format=json',expected=0)['task']
            self.assertEqual(stale['verification_invalidated_reason'], 'configuration_input_changed_or_unavailable')
            (root/'ruff.toml').write_text("[lint]\nselect = ['F401']\n")
            for field, value in [('schema_version','0.9.0'), ('task_scope','project_all'), ('delivery_decision','allow')]:
                forged = copy.deepcopy(scan); forged[field] = value
                self.assertFalse(schema.is_valid(forged), field)
            for n, mutate in enumerate([
                lambda r: r['task_binding'].update(path='unrelated.py'),
                lambda r: r['task_binding']['original_report'].update(report_sha256='0'*64),
                lambda r: r.update(unexpected_authority='approved'),
                lambda r: r['files'].append(copy.deepcopy(r['files'][0])),
            ]):
                forged = copy.deepcopy(scan)
                forged['run_id'] = f'lint-999-{n+1}'
                mutate(forged)
                path = root/'.codeguard/reports'/f"{forged['run_id']}.json"
                path.write_text(json.dumps(forged))
                sync = self.command('work', 'sync', str(root), '--format=json')
                self.assertEqual(sync['failed_reports'], 1, sync)
                path.unlink()
            human = subprocess.run([str(ROOT/'target/debug/codeguard'),'task','verify',task,str(root),'--ruff-tool',tool], capture_output=True, timeout=130, check=False)
            self.assertEqual(human.returncode, 3, human.stderr.decode())
            self.assertIn('复检范围', human.stdout.decode())
            self.assertIn('仅首次Python确认文件', human.stdout.decode())
            self.assertIn('原生状态', human.stdout.decode())
            # 丢失首次收据在启动工具前拒绝，不能靠当前源码重新生成首次证据。
            receipt = root/'.codeguard/state/consumed'/f"{original['run_id']}.json"
            receipt.unlink()
            marker = root/'tool-executed'
            fake = root/'fake-ruff'
            fake.write_text(f'#!/bin/sh\n/usr/bin/touch "{marker}"\nprintf "ruff 0.16.8\\n"\n')
            fake.chmod(0o700)
            rejected = self.command('task','verify',task,str(root),'--ruff-tool',str(fake),'--format=json')
            self.assertIsNone(rejected['native_scan'])
            self.assertIn(rejected['reason'], ['python_confirmation_original_not_synced', 'verification_event_invalid'])
            self.assertFalse(marker.exists())

if __name__ == '__main__':
    unittest.main()
