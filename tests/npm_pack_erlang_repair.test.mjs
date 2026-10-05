import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const bad = '-module(app).\nf( -> ok.\n';
const good = '-module(app).\nf() -> ok.\n';
const sha = bytes => createHash('sha256').update(bytes).digest('hex');

// 本地受控程序只检验 npm/Rust 协议编排；真实 OTP 通过单独的显式目标执行。
const fixture = `#!/bin/sh
case "$8" in *system_info*) printf 'OTP 28\\n'; exit 0;; esac
input=$(/bin/cat)
case "$input" in
  *'f( ->'*) printf '%s' '{"schema_version":"0.1.0","forms":2,"preprocessing":false,"diagnostics_truncated":false,"diagnostics":[{"line":2,"column":4,"rule_id":"erlang.syntax.error"}]}';;
  *) printf '%s' '{"schema_version":"0.1.0","forms":2,"preprocessing":false,"diagnostics_truncated":false,"diagnostics":[]}';;
esac
`;

function exercise(tarball, selectedTool, evidenceKind) {
  const sandbox = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'codeguard-npm-erlang-')));
  const scratch = path.join(sandbox, 'project');
  mkdirSync(scratch);
  try {
    const source = path.join(scratch, 'app.erl');
    writeFileSync(source, bad);
    let tool = selectedTool;
    if (!tool) {
      tool = path.join(sandbox, 'erl-tool');
      writeFileSync(tool, fixture, { mode: 0o700 });
    }
    function invoke(args, { input, human = false, expectedExit = args[0] === 'next' ? 0 : 3 } = {}) {
      const out = spawnSync('npm', [
        'exec', '--offline', '--yes', '--ignore-scripts', '--cache', path.join(sandbox, 'npm-cache'),
        '--package', tarball, '--', 'codeguard', ...args,
        ...(human ? [] : ['--format=json']),
      ], {
        cwd: scratch, input: input === undefined ? undefined : JSON.stringify(input),
        encoding: 'utf8', timeout: 120_000, maxBuffer: 1024 * 1024,
        env: { ...process.env, npm_config_audit: 'false', npm_config_fund: 'false' },
      });
      assert.equal(out.error, undefined, out.error?.message);
      assert.equal(out.signal, null);
      assert.equal(out.status, expectedExit, out.stderr);
      if (human) return out.stdout;
      try { return JSON.parse(out.stdout); } catch {
        assert.fail(`installed launcher did not return JSON: ${out.stderr}`);
      }
    }
    const initialized = invoke(['init', scratch, '--apply']);
    const installedVersion = invoke(['--version'], { expectedExit: 0 });
    const lint = invoke(['lint', 'erlang', source, '--erl-tool', tool, '--timeout', '30s']);
    const id = lint.task_id;
    assert.match(id ?? '', /^CG-B-[a-f0-9]{32}$/,
      'installed native-first lint must return a saved stable task, without preceding WASM');
    assert.equal(lint.schema_version, '0.3.0');
    assert.equal(lint.workspace_binding, 'bound');
    assert.equal(lint.native.status, 'diagnostics_observed');
    assert.equal(lint.native.version, 'OTP 28');
    assert.equal(lint.syntax_precheck, null, 'native result must take precedence over WASM');
    assert.equal(lint.task_sync_reason, null);

    const check = () => invoke(['check', 'all', scratch, '--erl-tool', tool, '--timeout', '30s']);
    const aggregate = check();
    assert.equal(aggregate.schema_version, '0.38.0');
    assert.deepEqual(aggregate.syntax_tasks, {
      failures: [], new_blockers: 0, status: 'synced_partial', tasks: [],
    }, 'native-covered Erlang must not create a second WASM task');
    const scan = aggregate.native_results.erlang_lint;
    assert.equal(scan.schema_version, '0.2.0');
    assert.equal(scan.files[0].task_id, id);
    assert.equal(scan.files[0].native.status, 'diagnostics_observed');
    assert.equal(aggregate.syntax_candidates.native_preferred_count, 1);
    assert.deepEqual(aggregate.syntax_candidates.observations, []);
    assert.equal(readdirSync(path.join(scratch, '.codeguard/tasks')).length, 1);

    const next = invoke(['next', scratch]);
    assert.equal(next.schema_version, '0.5.0');
    assert.equal(next.repair_brief.task_id, id);
    assert.equal(next.repair_brief.action_id, 'repair-source');
    assert.equal(next.repair_brief.native_column_unit, 'unicode_scalar');
    const ref = next.repair_brief.native_confirmation_ref;
    const raw = readFileSync(path.join(scratch, ref.report_ref));
    assert.equal(sha(raw), ref.report_sha256);
    const nativeFirst = JSON.parse(raw);
    assert.equal(nativeFirst.schema_version, '0.2.0');
    assert.deepEqual(nativeFirst.observations, [], 'native-first report must not invent a grammar');
    assert.equal(nativeFirst.native_evidence.target.source_sha256, sha(bad));
    assert.equal(nativeFirst.native_evidence.native.status, 'diagnostics_observed');
    const human = invoke(['lint', 'erlang', source, '--erl-tool', tool], { human: true });
    assert.ok(human.includes(id), 'human feedback must reference the same saved task');

    const verify = () => invoke(['task', 'verify', id, scratch, '--erl-tool', tool, '--timeout', '30s']);
    const present = verify();
    assert.equal(present.schema_version, '0.14.0');
    assert.equal(present.native_scan.schema_version, '0.3.0');
    assert.equal(present.native_scan.original_report.grammar_sha256, null);
    assert.equal(present.event_persisted, true);
    assert.equal(present.observation, 'still_blocked');
    assert.equal(present.native_scan.native.status, 'diagnostics_observed');
    assert.equal(present.native_scan.target.source_sha256, sha(bad));

    writeFileSync(source, good);
    const stale = invoke(['next', scratch]);
    assert.equal(stale.repair_brief.native_confirmation_status, 'stale');
    assert.deepEqual(stale.repair_brief.native_diagnostic_positions, []);
    const repaired = verify();
    assert.equal(repaired.native_scan.native.status, 'completed');
    assert.equal(repaired.native_scan.target.source_sha256, sha(good));
    assert.equal(repaired.observation, 'candidate_absent_unverified_policy');
    assert.equal(repaired.event_persisted, true);
    const fact = path.join(scratch, '.codeguard/findings', id, 'finding.json');
    assert.equal(JSON.parse(readFileSync(fact)).state, 'open');

    writeFileSync(source, bad);
    const recurrent = check();
    assert.equal(recurrent.native_results.erlang_lint.files[0].task_id, id);
    assert.equal(readdirSync(path.join(scratch, '.codeguard/tasks')).length, 1);
    const recurrenceNext = invoke(['next', scratch]);
    assert.equal(recurrenceNext.repair_brief.native_confirmation_status, 'diagnostics_observed');
    const hook = invoke(['hook', 'execute', scratch, '--erl-tool', tool, '--timeout', '30s'], {
      input: {
        schema_version: '1.0.0', report_type: 'hook_trigger_request',
        input: { event: 'repair_ready', changed_paths: [], task_id: id,
          write_outcome: 'confirmed', host_claims_blocking: false },
      },
    });
    assert.equal(hook.local_feedback.event_persisted, true);
    assert.equal(hook.local_feedback.native_confirmation_status, 'diagnostics_observed');
    assert.equal(hook.local_feedback.native_column_unit, 'unicode_scalar');
    const hookRef = hook.local_feedback.native_confirmation_ref;
    const hookRaw = readFileSync(path.join(scratch, hookRef.report_ref));
    assert.equal(sha(hookRaw), hookRef.report_sha256);
    assert.equal(JSON.parse(hookRaw).original_report.grammar_sha256, null);
    assert.equal(JSON.parse(readFileSync(fact)).state, 'open');

    // 保存失败要保留本次原生位置；不能向安装后的调用方提供虚构任务引用。
    const blockedProject = path.join(scratch, 'blocked');
    mkdirSync(blockedProject);
    invoke(['init', blockedProject, '--apply']);
    const blockedFile = path.join(blockedProject, 'other.erl');
    writeFileSync(blockedFile, bad);
    const reports = path.join(blockedProject, '.codeguard/reports');
    rmSync(reports, { recursive: true });
    writeFileSync(reports, 'not a report directory');
    const blocked = invoke(['lint', 'erlang', blockedFile, '--erl-tool', tool]);
    assert.equal(blocked.native.status, 'diagnostics_observed');
    assert.equal(blocked.task_id, null);
    assert.equal(blocked.task_status, 'incomplete');
    assert.equal(typeof blocked.task_sync_reason, 'string');

    return {
      evidence_kind: evidenceKind, tarball_sha256: sha(readFileSync(tarball)),
      installed_version: installedVersion, initialized, lint, aggregate,
      native_first: nativeFirst, next, human, present, stale, repaired,
      recurrent, recurrence_next: recurrenceNext, repair_ready: hook,
      persistence_failure: blocked,
    };
  } finally {
    rmSync(sandbox, { recursive: true, force: true });
  }
}

test('npm 安装入口保留 Erlang 原生首次证据与修复流程', async t => {
  const binary = process.env.CODEGUARD_WASM_BIN;
  assert.ok(binary, 'CODEGUARD_WASM_BIN is required');
  const packed = spawnSync(process.execPath, [
    path.join(root, 'scripts/pack-npm-local.mjs'), '--require-wasm', binary,
  ], { cwd: root, encoding: 'utf8', timeout: 120_000 });
  assert.equal(packed.status, 0, packed.stderr);
  const tarball = packed.stdout.trim();
  assert.ok(tarball.endsWith('.tgz'));
  const observations = [];
  await t.test('受控协议：原生优先、同一任务、陈旧位置撤回、保存失败不伪造 ID', () => {
    observations.push(exercise(tarball, null, 'controlled_native_protocol'));
  });
  await t.test('显式真实 OTP 28：安装后发现、修复、复检和再次发现', {
    skip: !process.env.CODEGUARD_ERL_BIN && 'requires explicitly selected real OTP 28',
  }, () => {
    observations.push(exercise(tarball, realpathSync(process.env.CODEGUARD_ERL_BIN), 'real_otp_28'));
  });
  if (process.env.CODEGUARD_NPM_ERLANG_ARTIFACT) {
    writeFileSync(process.env.CODEGUARD_NPM_ERLANG_ARTIFACT, JSON.stringify(observations, null, 2));
  }
});
