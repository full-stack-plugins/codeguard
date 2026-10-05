import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
// 受控工具检验安装后的协议编排；真实Ruby语法精度证据另行记录。
const fixture = `#!/bin/sh
if [ "$1" = --version ]; then printf 'ruby 2.6.10p210 (fixture) [test]\\n'; exit 0; fi
input=$(/bin/cat)
case "$input" in *'def f('*) printf '%s\\n' '-:1: syntax error, unexpected end-of-input IGNORE_GUARDS' >&2; exit 1;; esac
printf 'Syntax OK\\n'
`;

test('离线npm Ruby编辑、项目检查和原工具复检复用任务，不猜列号或自动关闭', () => {
  const binary = process.env.CODEGUARD_WASM_BIN;
  assert.ok(binary, 'CODEGUARD_WASM_BIN is required');
  const packed = spawnSync(process.execPath, [path.join(root, 'scripts/pack-npm-local.mjs'), '--require-wasm', binary], {
    cwd: root, encoding: 'utf8', timeout: 120_000,
  });
  assert.equal(packed.status, 0, packed.stderr);
  const tarball = packed.stdout.trim();
  const scratch = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'cg-npm-ruby-repair-')));
  const project = path.join(scratch, 'project');
  mkdirSync(project);
  try {
    const source = path.join(project, 'app.rb');
    const tool = path.join(scratch, 'ruby-tool');
    writeFileSync(source, 'def f(\n');
    writeFileSync(path.join(project, 'untouched.rb'), 'puts 1\n');
    writeFileSync(tool, fixture, { mode: 0o700 });
    function invoke(args, input, expected = 3) {
      const result = spawnSync('npm', ['exec', '--offline', '--yes', '--ignore-scripts', '--cache', path.join(scratch, 'cache'),
        '--package', tarball, '--', 'codeguard', ...args, '--format=json'], {
        cwd: project, input: input === undefined ? undefined : JSON.stringify(input), encoding: 'utf8', timeout: 120_000, maxBuffer: 1024 * 1024,
        env: { ...process.env, npm_config_audit: 'false', npm_config_fund: 'false' },
      });
      assert.equal(result.error, undefined, result.error?.message);
      assert.equal(result.status, expected, result.stderr);
      return JSON.parse(result.stdout);
    }
    const event = (kind = 'file_changed', id = null) => ({ schema_version: '1.0.0', report_type: 'hook_trigger_request', input: {
      event: kind, changed_paths: kind === 'file_changed' ? ['app.rb'] : [], task_id: id, write_outcome: 'confirmed', host_claims_blocking: false,
    } });
    const edit = () => invoke(['hook', 'execute', project, '--ruby-tool', tool, '--timeout', '30s'], event());
    const initialized = invoke(['init', project, '--apply']);
    const first = edit();
    assert.equal(first.schema_version, '0.19.0');
    const scan = first.local_feedback.ruby_lint;
    assert.equal(scan.source_file_count, 1);
    assert.equal(scan.files[0].path, 'app.rb');
    assert.equal(scan.files[0].native.status, 'diagnostics_observed');
    assert.deepEqual(scan.files[0].native.diagnostics, [{ line: 1, rule_id: 'ruby.syntax' }]);
    const id = scan.files[0].task_id;
    assert.match(id ?? '', /^CG-B-[a-f0-9]{32}$/);
    assert.equal(edit().local_feedback.ruby_lint.files[0].task_id, id);
    const aggregate = invoke(['check', 'ruby', project, '--ruby-tool', tool, '--timeout', '30s']);
    assert.equal(aggregate.schema_version, '0.51.0');
    assert.equal(aggregate.native_results.ruby_lint.files.find(f => f.path === 'app.rb').task_id, id);
    assert.deepEqual(aggregate.syntax_candidates.observations, []);
    const lint = invoke(['lint', 'ruby', source, '--ruby-tool', tool, '--timeout', '30s']);
    assert.equal(lint.task_id, id);
    const next = invoke(['next', project], undefined, 0);
    assert.equal(next.repair_brief.task_id, id);
    assert.equal(next.repair_brief.native_column_unit, 'unavailable');
    const conversation = invoke(['hook', 'claude', 'post-tool-use', project, '--ruby-tool', tool, '--timeout', '30s'], {
      hook_event_name: 'PostToolUse', cwd: project, tool_name: 'Edit', tool_input: { file_path: source, new_string: 'HOST_SECRET' }, tool_response: { success: true },
    }, 0);
    const context = conversation.hookSpecificOutput.additionalContext;
    assert.ok(context.includes(id) && context.includes('Ruby 第 1 行') && context.includes('--ruby-tool'));
    assert.ok(!context.includes('HOST_SECRET') && !context.includes('IGNORE_GUARDS'));
    assert.ok([...context].length <= 1200);
    const verify = () => invoke(['hook', 'execute', project, '--ruby-tool', tool, '--timeout', '30s'], event('repair_ready', id));
    const present = verify();
    assert.equal(present.schema_version, '0.20.0');
    assert.equal(present.local_feedback.native_confirmation_status, 'diagnostics_observed');
    assert.equal(present.local_feedback.event_persisted, true);
    assert.ok(present.local_feedback.native_confirmation_ref);
    writeFileSync(source, 'puts 1\n');
    const stale = invoke(['next', project], undefined, 0);
    assert.deepEqual(stale.repair_brief.native_diagnostic_positions, []);
    const repaired = verify();
    assert.equal(repaired.local_feedback.native_confirmation_status, 'completed');
    assert.equal(repaired.local_feedback.observation, 'candidate_absent_unverified_policy');
    assert.deepEqual(repaired.local_feedback.native_diagnostic_positions, []);
    const fact = path.join(project, '.codeguard/findings', id, 'finding.json');
    assert.equal(JSON.parse(readFileSync(fact)).state, 'open');
    writeFileSync(source, 'def f(\n');
    const recurrence = edit();
    assert.equal(recurrence.local_feedback.ruby_lint.files[0].task_id, id);
    assert.equal(JSON.parse(readFileSync(fact)).state, 'open');
    if (process.env.CODEGUARD_NPM_RUBY_ARTIFACT) writeFileSync(process.env.CODEGUARD_NPM_RUBY_ARTIFACT, JSON.stringify({ initialized, first, aggregate, lint, next, conversation, present, stale, repaired, recurrence }, null, 2));
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});
