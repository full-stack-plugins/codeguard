"""Diagnostics must preserve native failures under Windows legacy output codecs."""
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).resolve().parents[1] / 'scripts' / 'candidate-test.py'

class Diagnostics(unittest.TestCase):
    def test_unicode_output_preserves_each_exit_code(self):
        for code in [0, 9]:
            with self.subTest(code=code), tempfile.TemporaryDirectory(prefix='cg-diag-') as root:
                result = subprocess.run(
                    [sys.executable, str(SCRIPT), 'unicode-cli', sys.executable, '-c',
                     'import sys;sys.stdout.buffer.write("中文错误\\n".encode("utf-8"));raise SystemExit(%d)' % code],
                    cwd=root, env={**os.environ, 'PYTHONIOENCODING': 'cp1252'},
                    capture_output=True, timeout=15)
                self.assertEqual(result.returncode, code, result.stderr)
                output = result.stdout.decode('utf-8')
                self.assertIn('中文错误', output)
                self.assertEqual('::error title=Candidate unicode-cli failed::' in output, bool(code))
                self.assertEqual(pathlib.Path(root, 'candidate-evidence/unicode-cli.log').read_bytes(),
                                 '中文错误\n'.encode('utf-8'))

if __name__ == '__main__':
    unittest.main()
