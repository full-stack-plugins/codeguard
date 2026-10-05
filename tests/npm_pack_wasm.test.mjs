import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
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
    const extracted = spawnSync('tar', ['-xzf', tarball, '-C', scratch], { encoding: 'utf8', timeout: 30_000 });
    assert.equal(extracted.status, 0, extracted.stderr);
    const manifest = JSON.parse(readFileSync(path.join(root, 'grammars', 'manifest.json')));
    const codegraphLicense = readFileSync(path.join(scratch, 'package', 'grammar-licenses', manifest.codegraph_license));
    assert.equal(createHash('sha256').update(codegraphLicense).digest('hex'), manifest.codegraph_license_sha256);
    for (const asset of manifest.assets) {
      const license = readFileSync(path.join(scratch, 'package', 'grammar-licenses', asset.license));
      assert.equal(createHash('sha256').update(license).digest('hex'), asset.license_sha256, asset.language);
    }
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
      assert.equal(report.grammar_sha256, manifest.assets.find(asset => asset.language === language).sha256);
    }
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});

test('本地离线 npm 包通过统一 check all 调用全部 32 份候选', () => {
  assert.ok(wasmBinary, 'CODEGUARD_WASM_BIN is required');
  const packed = spawnSync(process.execPath, [packer, '--require-wasm', wasmBinary], {
    cwd: root, encoding: 'utf8', timeout: 120_000,
  });
  assert.equal(packed.status, 0, packed.stderr);
  const tarball = packed.stdout.trim();
  const scratch = realpathSync(mkdtempSync(path.join(os.tmpdir(), 'codeguard-npm-wasm-all-')));
  const samples = [
    ['a.ets', "@Component struct C { build() { Text('hi') } }"],
    ['a.c', 'int main(void) { return 0; }'],
    ['a.cfm', '<cfquery name="q">SELECT #x# FROM users</cfquery>'],
    ['a.cfs', 'component { function f() { return 1; } }'],
    ['a.cbl', '       IDENTIFICATION DIVISION.\n       PROGRAM-ID. HELLO.\n'],
    ['a.cpp', 'int main() { return 0; }'],
    ['a.cs', 'class C { static int F() { return 1; } }'],
    ['a.dart', "String greet(String name) => 'Hello $name';"],
    ['a.erl', '-module(hello).\nhello() -> ok.\n'],
    ['a.go', 'package main\nfunc main() {}\n'],
    ['a.java', 'class A {}'],
    ['a.js', 'const value = 1;'],
    ['a.kt', 'fun main() { val x = 1 }'],
    ['a.lua', 'local x = 1\n'],
    ['a.luau', 'local x: number = 1\n'],
    ['a.nix', 'let x = 1; in x'],
    ['a.m', '@interface Foo : NSObject\n@end\n'],
    ['a.pas', "program Hello;\nbegin\n  writeln('Hi');\nend.\n"],
    ['a.php', '<?php function f() { return 1; }'],
    ['a.py', 'x = 1\n'],
    ['a.r', 'x <- 1\n'],
    ['a.rb', 'def f; 1; end\n'],
    ['a.rs', 'fn main() { let x = 1; }'],
    ['a.scala', 'object Main { def f(x: Int): Int = x + 1 }'],
    ['a.sol', 'pragma solidity ^0.8.20;\ncontract Vault {}\n'],
    ['a.swift', 'func f() -> Int { return 1 }'],
    ['a.tf', 'resource "x" "y" { foo = "bar" }'],
    ['a.tsx', 'const C = () => <div />;'],
    ['a.ts', 'const value: number = 1;'],
    ['a.vb', 'Public Class C\n    Public Function F() As Integer\n        Return 1\n    End Function\nEnd Class\n'],
    ['a.zig', 'const Empty = struct {};\n'],
  ];
  try {
    // 此场景验收候选回退，不应继承 runner 的原生编译器。
    const runtimePath = path.join(scratch, 'runtime');
    mkdirSync(runtimePath);
    symlinkSync(process.execPath, path.join(runtimePath, 'node'));
    symlinkSync('/bin/sh', path.join(runtimePath, 'sh'));
    const npmPath = realpathSync(path.join(path.dirname(process.execPath), process.platform === 'win32' ? 'npm.cmd' : 'npm'));
    const observed = new Set();
    samples.forEach(([filename, source], index) => {
      const group = path.join(scratch, `group-${Math.floor(index / 8)}`);
      mkdirSync(group, { recursive: true });
      writeFileSync(path.join(group, filename), source);
    });
    for (let groupIndex = 0; groupIndex < 4; groupIndex++) {
      const checked = spawnSync(npmPath, [
        'exec', '--offline', '--yes', '--cache', path.join(scratch, 'cache'),
        '--package', tarball, '--', 'codeguard', 'check', 'all', path.join(scratch, `group-${groupIndex}`),
        '--format=json', '--timeout', '120s',
      ], { cwd: root, env: { ...process.env, PATH: runtimePath }, encoding: 'utf8', timeout: 150_000, maxBuffer: 4 * 1024 * 1024 });
      assert.equal(checked.status, 3, `group ${groupIndex}: ${checked.stderr}`);
      const report = JSON.parse(checked.stdout);
      assert.equal(report.delivery_decision, 'incomplete');
      assert.equal(report.syntax_candidates.skipped_count, 0, `group ${groupIndex}`);
      for (const item of report.syntax_candidates.observations) {
        if (item.status === 'candidate_observed') observed.add(item.language);
      }
    }
    const manifest = JSON.parse(readFileSync(path.join(root, 'grammars', 'manifest.json')));
    assert.deepEqual([...observed].sort(), manifest.assets.map(asset => asset.language).sort());
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
});
