"""用显式提供的历史 CLI 和当前 CLI 验收真实文档尝试迁移，不改写历史。"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument('--legacy-cli', type=Path, required=True)
parser.add_argument('--legacy-sha256', required=True)
parser.add_argument('--current-cli', type=Path, required=True)
parser.add_argument('--clang-tool', type=Path, required=True)
args = parser.parse_args()
legacy = args.legacy_cli.resolve(strict=True)
current = args.current_cli.resolve(strict=True)
assert hashlib.sha256(legacy.read_bytes()).hexdigest() == args.legacy_sha256
assert b'clang-documentation-structure-attempt-input-v1' in legacy.read_bytes()
assert b'clang-documentation-structure-attempt-input-v2' not in legacy.read_bytes()
assert b'clang-documentation-structure-attempt-input-v2' in current.read_bytes()


def call(binary, *words):
    result = subprocess.run([str(binary), *map(str, words), '--format=json'],
                            capture_output=True, timeout=30, check=False)
    assert result.returncode in (0, 3), (result.returncode, result.stderr.decode())
    return json.loads(result.stdout)


with tempfile.TemporaryDirectory(prefix='cg-engine-migration-') as temporary:
    root = Path(temporary).resolve()
    source = root / 'api.c'
    source.write_text('int f(int x) { return x; }\n')
    call(legacy, 'init', root, '--apply')
    scan = call(legacy, 'comments', 'c', source, '--workspace', root,
                '--clang-tool', args.clang_tool, '--standard', 'c11')
    task_id = scan['structural_workbench']['task_ids'][0]

    def task(binary, *words, options=()):
        return call(binary, 'task', *words, task_id, root, *options)

    lease = task(legacy, 'claim', options=('--owner', 'engine-upgrade'))
    auth = ('--owner', 'engine-upgrade', '--lease-token', lease['lease_token'])

    def attempt(binary):
        started = task(binary, 'attempt', 'start',
                       options=(*auth, '--action-id', 'repair-source'))
        assert 'attempt_id' in started
        task(binary, 'attempt', 'finish', options=(
            *auth, '--attempt-id', started['attempt_id'],
            '--outcome', 'ready-to-verify', '--note-code', 'source_edit'))
        recheck = task(binary, 'verify', options=(*auth, '--clang-tool', args.clang_tool))
        assert recheck['observation'] == 'still_present'
        assert recheck['event_persisted'] is True
        return started['attempt_id']

    old_attempt = attempt(legacy)
    old_history = task(legacy, 'show')['task']['history']
    assert old_history['attempt_count'] == 1
    assert old_history['no_progress_count'] == 1
    saved_events = {path: path.read_bytes() for path in root.rglob('events/*.json')}
    assert saved_events
    migrated = task(current, 'show')['task']
    assert migrated['history']['attempt_count'] == 0
    assert migrated['history']['no_progress_count'] == 0
    assert old_attempt in [row['attempt_id'] for row in migrated['history']['recent']]
    assert migrated['disposition'] == 'actionable'
    new_attempt = attempt(current)
    assert new_attempt != old_attempt
    history = task(current, 'show')['task']['history']
    assert history['attempt_count'] == 1
    assert history['no_progress_count'] == 1
    assert {old_attempt, new_attempt}.issubset({row['attempt_id'] for row in history['recent']})
    for path, data in saved_events.items():
        assert path.read_bytes() == data, path
    fact = json.loads((root / '.codeguard' / 'findings' / task_id / 'finding.json').read_text())
    assert fact['state'] == 'open'
    print('PASS: historical events preserved; current engine requires its own original-tool recheck; finding remains open')
