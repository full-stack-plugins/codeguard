import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('离线 npm 包贯通编辑任务与原生复检，未批准时不关闭', () => {
  const binary = process.env.CODEGUARD_WASM_BIN;
  assert.ok(binary, 'CODEGUARD_WASM_BIN is required');
  const packed = spawnSync(process.execPath, [
    path.join(root, 'scripts', 'pack-npm-local.mjs'), '--require-wasm', binary,
  ], { cwd: root, encoding: 'utf8', timeout: 120_000 });
  assert.equal(packed.status, 0, packed.stderr);
  const tarball = packed.stdout.trim();
  assert.ok(tarball.endsWith('.tgz'));
  const scratch = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'codeguard-npm-repair-')));
  try {
    const file = path.join(scratch, 'app.zig');
    const bad = 'pub fn main( void {\n';
    const good = 'pub fn main() void {}\n';
    writeFileSync(file, bad);
    function invoke(args, input) {
      const result = spawnSync('npm', [
        'exec', '--offline', '--yes', '--ignore-scripts', '--cache', path.join(scratch, 'npm-cache'),
        '--package', tarball, '--', 'codeguard', ...args, '--format=json',
      ], {
        cwd: scratch, input: input === undefined ? undefined : JSON.stringify(input),
        encoding: 'utf8', timeout: 120_000, maxBuffer: 1024 * 1024,
        env: { ...process.env, npm_config_audit: 'false', npm_config_fund: 'false' },
      });
      assert.equal(result.error, undefined, result.error?.message);
      assert.equal(result.signal, null);
      let value;
      try { value = JSON.parse(result.stdout); } catch {
        assert.fail(`npm launcher did not return JSON: ${result.stderr}`);
      }
      return { exit: result.status, value };
    }
    const initialized = invoke(['init', scratch, '--apply']);
    assert.equal(initialized.exit, 3);
    const changed = {
      schema_version: '1.0.0', report_type: 'hook_trigger_request',
      input: {
        event: 'file_changed', changed_paths: ['app.zig'], task_id: null,
        write_outcome: 'confirmed', host_claims_blocking: false,
      },
    };
    const edit = () => invoke(['hook', 'execute', scratch, '--timeout=30s'], changed);
    const first = edit();
    assert.equal(first.exit, 3);
    assert.equal(first.value.delivery_decision, 'not_evaluated');
    assert.ok(first.value.local_feedback?.syntax_tasks,
      'installed package must connect edit feedback to syntax tasks');
    const tasks = first.value.local_feedback.syntax_tasks;
    assert.equal(tasks?.status, 'synced_partial', 'installed package must persist syntax tasks');
    const id = tasks.tasks[0].task_id;
    assert.match(id, /^CG-B-[a-f0-9]+$/);
    const repeated = edit();
    assert.equal(repeated.exit, 3);
    assert.equal(repeated.value.local_feedback.syntax_tasks.tasks[0].task_id, id);

    const conversation = invoke([
      'hook', 'claude', 'post-tool-use', scratch, '--timeout=30s',
    ], {
      hook_event_name: 'PostToolUse', cwd: scratch, tool_name: 'Edit',
      tool_input: {
        file_path: file, old_string: 'old', new_string: 'HOST_SOURCE_MUST_NOT_BE_ECHOED',
      },
      tool_response: { success: true },
    });
    assert.equal(conversation.exit, 0);
    assert.equal(conversation.value.hookSpecificOutput.hookEventName, 'PostToolUse');
    const context = conversation.value.hookSpecificOutput.additionalContext;
    assert.ok(context.includes(id));
    assert.ok(context.includes('codeguard task show'));
    assert.ok([...context].length <= 1200);
    assert.ok(!context.includes('HOST_SOURCE_MUST_NOT_BE_ECHOED'));

    // 受控工具只验证安装后编排；真实 Zig 验收另有独立用例。
    const tool = path.join(scratch, 'zig-tool');
    writeFileSync(tool, `#!/bin/sh
if [ "$1" = version ]; then printf '0.16.0\\n'; exit 0; fi
while IFS= read -r line; do
  case "$line" in *'main( void'*) printf '<stdin>:1:13: error: expected token\\n' >&2; exit 1;; esac
done
exit 0
`, { mode: 0o700 });
    const verify = () => invoke([
      'task', 'verify', id, scratch, '--zig-tool', tool, '--timeout', '30s',
    ]);
    const present = verify();
    assert.equal(present.exit, 3);
    assert.equal(present.value.event_persisted, true);
    assert.equal(present.value.observation, 'still_blocked');
    assert.equal(present.value.native_scan.native.status, 'diagnostics_observed');
    const next = invoke(['next', scratch]);
    assert.equal(next.value.repair_brief.disposition, 'actionable');
    assert.equal(next.value.repair_brief.native_diagnostic_positions[0].column, 13);
    assert.equal(next.value.repair_brief.recheck_argv.at(-1), tool);
    assert.equal(next.value.repair_brief.native_confirmation_ref.run_id,
      present.value.native_scan.run_id);

    writeFileSync(file, good);
    const stale = invoke(['next', scratch]);
    assert.equal(stale.value.repair_brief.disposition, 'verification_required');
    assert.deepEqual(stale.value.repair_brief.native_diagnostic_positions, []);
    const repaired = verify();
    assert.equal(repaired.exit, 3);
    assert.equal(repaired.value.event_persisted, true);
    assert.equal(repaired.value.observation, 'candidate_absent_unverified_policy');
    assert.equal(repaired.value.native_scan.native.status, 'completed');
    const factPath = path.join(scratch, '.codeguard', 'findings', id, 'finding.json');
    assert.equal(JSON.parse(readFileSync(factPath)).state, 'open');
    const cleanCandidate = edit();
    assert.equal(cleanCandidate.value.local_feedback.candidate_recovery_count, 0);
    assert.equal(JSON.parse(readFileSync(factPath)).state, 'open');

    writeFileSync(file, bad);
    const again = edit();
    assert.equal(again.value.local_feedback.syntax_tasks.tasks[0].task_id, id);
    assert.equal(verify().value.observation, 'still_blocked');
    assert.equal(JSON.parse(readFileSync(factPath)).state, 'open');
    if (process.env.CODEGUARD_NPM_REPAIR_ARTIFACT) {
      writeFileSync(process.env.CODEGUARD_NPM_REPAIR_ARTIFACT, JSON.stringify({
        initialized: initialized.value, edit: first.value, repeated: repeated.value,
        conversation: conversation.value, present: present.value, next: next.value,
        stale: stale.value, repaired: repaired.value, clean_candidate: cleanCandidate.value,
        repeated_problem: again.value,
      }, null, 2));
    }
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});
