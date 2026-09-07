import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

test('pack only supplied platforms and preserve the complete CI matrix', t => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'canisend-npm-pack-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const targets = ['aarch64-apple-darwin', 'x86_64-apple-darwin', 'x86_64-unknown-linux-gnu', 'x86_64-unknown-linux-musl', 'x86_64-pc-windows-msvc'];
  const bundles = targets.map(target => {
    const directory = path.join(root, target);
    fs.mkdirSync(directory);
    for (const file of ['LICENSE', 'THIRD_PARTY_NOTICES.md', 'TYPST-ASSETS-LICENSE', 'TYPST-ASSETS-NOTICE', target.includes('windows') ? 'canisend.exe' : 'canisend']) {
      fs.writeFileSync(path.join(directory, file), 'isolated packaging fixture\n');
    }
    fs.writeFileSync(path.join(directory, 'TARGET'), target);
    fs.writeFileSync(path.join(directory, 'RELEASE.json'), JSON.stringify({ ok: true, data: { product: 'canisend', target, version: '1.0.0-beta.3', git_revision: 'fixture' } }));
    return directory;
  });
  for (const count of [1, targets.length]) {
    if (count === 1) fs.writeFileSync(path.join(bundles[0], 'SOURCE.tar.gz'), 'source archive fixture');
    else fs.rmSync(path.join(bundles[0], 'SOURCE.tar.gz'));
    const output = path.join(root, `packages-${count}`);
    execFileSync(process.execPath, [fileURLToPath(new URL('pack.mjs', import.meta.url)), output, ...bundles.slice(0, count)], {
      env: { ...process.env, npm_config_cache: path.join(root, 'cache'), npm_config_offline: 'true' },
      stdio: 'pipe',
    });
    const entry = JSON.parse(fs.readFileSync(path.join(output, 'canisend/package.json')));
    const report = JSON.parse(fs.readFileSync(path.join(output, 'packages.json')));
    assert.equal(Object.keys(entry.optionalDependencies).length, count);
    assert.equal(report.complete, count === targets.length);
    assert.equal(report.archives.length, count + 1);
    for (const name of Object.keys(entry.optionalDependencies)) {
      assert.ok(report.archives.includes(`${name}-${entry.version}.tgz`));
    }
    if (count === 1) {
      assert.deepEqual(entry.os, ['darwin']);
      assert.deepEqual(entry.cpu, ['arm64']);
      assert.deepEqual(entry.optionalDependencies, { 'canisend-darwin-arm64': entry.version });
      assert.equal(fs.readFileSync(path.join(output, 'canisend/SOURCE.tar.gz'), 'utf8'), 'source archive fixture');
      assert.ok(!fs.readFileSync(path.join(output, 'canisend/README.md'), 'utf8').includes('x86_64-unknown-linux-gnu'));
    } else {
      assert.equal(entry.os, undefined);
      assert.equal(entry.cpu, undefined);
      assert.equal(fs.existsSync(path.join(output, 'canisend/SOURCE.tar.gz')), false);
      assert.deepEqual(JSON.parse(fs.readFileSync(path.join(output, 'canisend-linux-x64-gnu/package.json'))).libc, ['glibc']);
    }
  }
});
