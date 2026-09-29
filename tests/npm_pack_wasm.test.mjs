import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const packer = path.join(root, 'scripts', 'pack-npm-local.mjs');
const wasmBinary = process.env.CODEGUARD_WASM_BIN;
const defaultBinary = process.env.CODEGUARD_NO_WASM_BIN;

test('打包器拒绝未包含 WASM 的二进制', () => {
  assert.ok(defaultBinary, 'CODEGUARD_NO_WASM_BIN is required');
  const result = spawnSync(process.execPath, [packer, '--require-wasm', defaultBinary], {
    cwd: root, encoding: 'utf8', timeout: 30_000,
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /WASM capability probe failed/);
});

test('本地离线 npm 包实际运行固定 Zig 与 Dart WASM', () => {
  assert.ok(wasmBinary, 'CODEGUARD_WASM_BIN is required');
  const packed = spawnSync(process.execPath, [packer, '--require-wasm', wasmBinary], {
    cwd: root, encoding: 'utf8', timeout: 120_000,
  });
  assert.equal(packed.status, 0, packed.stderr);
  const tarball = packed.stdout.trim();
  assert.ok(tarball.endsWith('.tgz'));
  const scratch = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'codeguard-npm-wasm-')));
  try {
    const inventory = spawnSync('npm', [
      'exec', '--offline', '--yes', '--cache', path.join(scratch, 'cache'),
      '--package', tarball, '--', 'codeguard', 'grammar', 'status', '--format=json',
    ], { cwd: root, encoding: 'utf8', timeout: 120_000 });
    assert.equal(inventory.status, 3, inventory.stderr);
    const assets = JSON.parse(inventory.stdout);
    assert.equal(assets.candidate_count, 32);
    assert.equal(assets.released_count, 0);
    for (const [language, filename, source] of [
      ['zig', 'sample.zig', 'const Empty = struct {};\n'],
      ['dart', 'sample.dart', "String greet(String name) => 'Hello $name';\n"],
    ]) {
      const file = path.join(scratch, filename);
      writeFileSync(file, source);
      const probe = spawnSync('npm', [
        'exec', '--offline', '--yes', '--cache', path.join(scratch, 'cache'),
        '--package', tarball, '--', 'codeguard', 'grammar', 'probe', language, file,
        '--format=json',
      ], { cwd: root, encoding: 'utf8', timeout: 120_000 });
      assert.equal(probe.status, 3, probe.stderr);
      const report = JSON.parse(probe.stdout);
      assert.equal(report.report_type, 'grammar_candidate_probe');
      assert.equal(report.language, language);
      assert.equal(report.grammar_qualified, false);
      assert.equal(report.native.status, 'not_run');
      assert.equal(report.delivery_decision, 'not_evaluated');
      const manifest = JSON.parse(readFileSync(path.join(root, 'grammars', 'manifest.json')));
      assert.equal(report.grammar_sha256, manifest.assets.find(asset => asset.language === language).sha256);
    }
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});
