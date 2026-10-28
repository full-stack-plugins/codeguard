#!/usr/bin/env node
// Actual private candidate pack/install acceptance. No registry access, lifecycle
// scripts, global installation or public publishing; failures are never skipped.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { npmCommand, packageTarget } from '../scripts/pack-npm-local.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
assert.equal(process.argv.length, 3, 'Usage: node tests/npm_candidate_install.mjs <candidate-binary>');
const binary = path.resolve(process.argv[2]);
function run(command, args, options = {}) {
  const result = spawnSync(command, args, {cwd: root, encoding:'utf8', timeout:180_000,
    maxBuffer:4*1024*1024, shell:false, ...options});
  assert.ifError(result.error);
  assert.equal(result.status, 0, `${command} failed: ${result.stderr}\n${result.stdout}`);
  return result.stdout;
}
const original = JSON.parse(run(binary, ['--version','--format','json']));
assert.equal(original.target, packageTarget(`${process.platform}-${process.arch}`));
assert.match(original.build_identity, /^[a-f0-9]{40}$/);
const packed = run(process.execPath, [path.join(root,'scripts/pack-npm-local.mjs'),'--require-wasm',binary]);
const tarball = packed.trim().split(/\r?\n/).at(-1);
assert(path.isAbsolute(tarball));
const profile = mkdtempSync(path.join(os.tmpdir(), 'codeguard candidate & install-'));
try {
  const npm = npmCommand();
  run(npm.command, [...npm.args,'install','--prefix',profile,'--offline','--ignore-scripts',
    '--no-audit','--no-fund','--package-lock=false',tarball],
    {cwd:profile, env:{...process.env,npm_config_cache:path.join(profile,'cache')}});
  const installed = path.join(profile,'node_modules','@full-stack-plugins','codeguard');
  const manifest = JSON.parse(readFileSync(path.join(installed,'package.json'),'utf8'));
  assert.equal(manifest.private,true);
  assert.equal(manifest.name,'@full-stack-plugins/codeguard');
  assert.deepEqual(manifest.os,[process.platform]);
  assert.deepEqual(manifest.cpu,[process.arch]);
  assert.equal(manifest.bin.codeguard,'./codeguard.cjs');
  const actual = JSON.parse(run(process.execPath,[path.join(installed,'codeguard.cjs'),'--version','--format','json'],{cwd:profile}));
  assert.deepEqual(actual,original);
  const hash = file => createHash('sha256').update(readFileSync(file)).digest('hex');
  const installedNative = path.join(installed,'native',process.platform==='win32'?'codeguard.exe':'codeguard');
  assert.equal(hash(installedNative),hash(binary));
  process.stdout.write(JSON.stringify({status:'passed',platform:process.platform,architecture:process.arch,
    identity:actual,tarball,tarball_sha256:hash(tarball),native_sha256:hash(binary),
    install:'temporary offline npm install with lifecycle scripts disabled',
    invocation:'installed npm manifest JavaScript entry via Node; no shell shim',
    wasm_gate:'32 pinned grammar assets/licenses and real Zig worker required'},null,2)+'\n');
} finally { rmSync(profile,{recursive:true,force:true}); }
