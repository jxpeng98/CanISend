'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const { test } = require('node:test');
const source = fs.readFileSync(`${__dirname}/canisend.cjs`, 'utf8');
function run(platform, arch, glibc, result = { status: 0 }, missing = false) {
  let requested;
  let spawned;
  let killed;
  const process = { platform, arch, pid: 123, argv: ['node', 'cli', 'mcp', 'serve'],
    report: { getReport: () => ({ header: { glibcVersionRuntime: glibc } }) },
    kill: (...args) => { killed = args; } };
  const require = () => ({ spawnSync: (...args) => { spawned = args; return result; } });
  require.resolve = name => { requested = name; if (missing) throw Error('missing native package'); return '/native'; };
  vm.runInNewContext(source, { require, process, console: { error() {} } });
  return { requested, spawned, killed, code: process.exitCode };
}
test('select embedded native binaries and forward arguments/stdin/stdout without a shell', () => {
  for (const [os, arch, glibc, suffix] of [
    ['darwin', 'arm64', undefined, 'darwin-arm64/canisend'],
    ['darwin', 'x64', undefined, 'darwin-x64/canisend'],
    ['linux', 'x64', '2.39', 'linux-x64-gnu/canisend'],
    ['linux', 'x64', undefined, 'linux-x64-musl/canisend'],
    ['win32', 'x64', undefined, 'win32-x64/canisend.exe'],
  ]) {
    const result = run(os, arch, glibc);
    assert.equal(result.requested, `./native/${suffix}`);
    assert.equal(JSON.stringify(result.spawned), JSON.stringify(['/native', ['mcp', 'serve'], { stdio: 'inherit' }]));
    assert.equal(result.code, 0);
  }
});
test('preserve failures and termination; report missing native binaries', () => {
  assert.equal(run('darwin', 'arm64', null, { status: 7 }).code, 7);
  assert.deepEqual(run('darwin', 'arm64', null, { signal: 'SIGTERM' }).killed, [123, 'SIGTERM']);
  assert.equal(run('darwin', 'arm64', null, { error: Error('spawn failed') }).code, 1);
  assert.equal(run('darwin', 'arm64', null, undefined, true).code, 1);
});
