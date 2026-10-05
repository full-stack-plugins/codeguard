import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
// 受控工具检验安装后的协议编排；真实ShellCheck语法精度证据另行记录。
const fixture = `#!/bin/sh
if [ "$1" = --version ]; then printf 'ShellCheck - shell script analysis tool\\nversion: 0.11.0\\nlicense: GNU General Public License, version 3\\nwebsite: https://www.shellcheck.net\\n'; exit 0; fi
input=$(/bin/cat)
case "$input" in *'echo $1'*) printf '%s\\n' '{"comments":[{"file":"-","line":2,"endLine":2,"column":6,"endColumn":8,"level":"info","code":2086,"message":"IGNORE_GUARDS","fix":null}]}'; exit 1;; esac
printf '%s\\n' '{"comments":[]}'
`;

test('离线npm ShellCheck编辑、项目检查和原工具复检复用任务，使用原始字符列且不自动关闭', () => {
  const binary = process.env.CODEGUARD_WASM_BIN;
  assert.ok(binary, 'CODEGUARD_WASM_BIN is required');
  const packed = spawnSync(process.execPath, [path.join(root, 'scripts/pack-npm-local.mjs'), '--require-wasm', binary], {
    cwd: root, encoding: 'utf8', timeout: 120_000,
  });
  assert.equal(packed.status, 0, packed.stderr);
  const tarball = packed.stdout.trim();
  const scratch = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'cg-npm-shell-repair-')));
  const project = path.join(scratch, 'project');
  mkdirSync(project);
  try {
    const source = path.join(project, 'app.sh');
    const tool = path.join(scratch, 'shell-tool');
    writeFileSync(source, '#!/bin/bash\necho $1\n');
    writeFileSync(path.join(project, 'untouched.sh'), '#!/bin/bash\necho "$1"\n');
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
      event: kind, changed_paths: kind === 'file_changed' ? ['app.sh'] : [], task_id: id, write_outcome: 'confirmed', host_claims_blocking: false,
    } });
    const edit = () => invoke(['hook', 'execute', project, '--shellcheck-tool', tool, '--timeout', '30s'], event());
    const initialized = invoke(['init', project, '--apply']);
    const first = edit();
    assert.equal(first.schema_version, '0.21.0');
    const scan = first.local_feedback.shell_lint;
    assert.equal(scan.source_file_count, 1);
    assert.equal(scan.files[0].path, 'app.sh');
    assert.equal(scan.files[0].native.status, 'diagnostics_observed');
    assert.equal(scan.files[0].native.diagnostics[0].rule_id, 'SC2086');
    assert.equal(scan.files[0].native.diagnostics[0].column, 6);
    const id = scan.files[0].workbench.task_ids[0];
    assert.match(id ?? '', /^CG-[a-f0-9]{32}$/);
    assert.equal(edit().local_feedback.shell_lint.files[0].workbench.task_ids[0], id);
    const aggregate = invoke(['check', 'shell', project, '--shellcheck-tool', tool, '--timeout', '30s']);
    assert.equal(aggregate.schema_version, '0.52.0');
    assert.equal(aggregate.native_results.shell_lint.files.find(f => f.path === 'app.sh').workbench.task_ids[0], id);
    assert.deepEqual(aggregate.syntax_candidates.observations, []);
    const lint = invoke(['lint', 'shell', source, '--dialect', 'bash', '--shellcheck-tool', tool, '--timeout', '30s']);
    assert.equal(lint.workbench.task_ids[0], id);
    const next = invoke(['next', project], undefined, 0);
    assert.equal(next.repair_brief.task_id, id);
    assert.ok(next.repair_brief.recheck_argv.includes('--shellcheck-tool'));
    const conversation = invoke(['hook', 'claude', 'post-tool-use', project, '--shellcheck-tool', tool, '--timeout', '30s'], {
      hook_event_name: 'PostToolUse', cwd: project, tool_name: 'Edit', tool_input: { file_path: source, new_string: 'HOST_SECRET' }, tool_response: { success: true },
    }, 0);
    const context = conversation.hookSpecificOutput.additionalContext;
    assert.ok(context.includes(id) && context.includes('Shell 第 2 行') && context.includes('--shellcheck-tool'));
    assert.ok(!context.includes('HOST_SECRET') && !context.includes('IGNORE_GUARDS'));
    assert.ok([...context].length <= 1200);
    const verify = () => invoke(['hook', 'execute', project, '--shellcheck-tool', tool, '--timeout', '30s'], event('repair_ready', id));
    const present = verify();
    assert.equal(present.local_feedback.observation, 'still_present');
    assert.equal(present.local_feedback.event_persisted, true);
    writeFileSync(source, '#!/bin/bash\necho "$1"\n');
    const stale = invoke(['next', project], undefined, 0);
    assert.notEqual(stale.disposition, "no_work");
    const repaired = verify();
    assert.equal(repaired.local_feedback.observation, 'candidate_absent_unverified_policy');
    const fact = path.join(project, '.codeguard/findings', id, 'finding.json');
    assert.equal(JSON.parse(readFileSync(fact)).state, 'open');
    writeFileSync(source, '#!/bin/bash\necho $1\n');
    const recurrence = edit();
    assert.equal(recurrence.local_feedback.shell_lint.files[0].workbench.task_ids[0], id);
    assert.equal(JSON.parse(readFileSync(fact)).state, 'open');
    const marker = path.join(scratch, 'failed-write-executed');
    const shellQuotedMarker = "'" + marker.replaceAll("'", "'\\''") + "'";
    writeFileSync(tool, `#!/bin/sh\n/usr/bin/touch ${shellQuotedMarker}\nexit 7\n`);
    const failedEvent = event(); failedEvent.input.write_outcome = 'failed';
    const failed = invoke(['hook', 'execute', project, '--shellcheck-tool', tool, '--timeout', '30s'], failedEvent);
    assert.equal(failed.execution, 'not_run');
    assert.equal(failed.reason, 'write_failed');
    assert.equal(existsSync(marker), false);
    if (process.env.CODEGUARD_NPM_SHELL_ARTIFACT) writeFileSync(process.env.CODEGUARD_NPM_SHELL_ARTIFACT, JSON.stringify({ initialized, first, aggregate, lint, next, conversation, present, stale, repaired, recurrence, failed }, null, 2));
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});
