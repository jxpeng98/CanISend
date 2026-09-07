// Package already staged native bundles; no downloads or publication here.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
const platforms = {
  'aarch64-apple-darwin': ['darwin', 'arm64'],
  'x86_64-apple-darwin': ['darwin', 'x64'],
  'x86_64-unknown-linux-gnu': ['linux', 'x64', 'gnu'],
  'x86_64-unknown-linux-musl': ['linux', 'x64', 'musl'],
  'x86_64-pc-windows-msvc': ['win32', 'x64'],
};
const [destination, ...bundles] = process.argv.slice(2);
if (!destination || !bundles.length) throw Error('usage: node pack.mjs NEW_OUTPUT STAGED_BUNDLE...');
const output = path.resolve(destination);
const seen = new Set();
const inputs = bundles.map(bundle => {
  const identity = JSON.parse(fs.readFileSync(path.join(bundle, 'RELEASE.json')));
  const target = fs.readFileSync(path.join(bundle, 'TARGET'), 'utf8').trim();
  if (!identity.ok || identity.data.product !== 'canisend' || identity.data.target !== target || !platforms[target]) throw Error('invalid bundle identity');
  if (seen.has(target)) throw Error(`duplicate target: ${target}`);
  seen.add(target);
  return { bundle, target, ...identity.data };
});
const version = inputs[0].version;
if (!/^\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?$/.test(version) || inputs.some(x => x.version !== version || x.git_revision !== inputs[0].git_revision)) throw Error('bundle versions or revisions differ');
fs.mkdirSync(output); // Refuse overwrite.
const common = { version, license: 'GPL-3.0-only', repository: { type: 'git', url: 'git+https://github.com/jxpeng98/CanISend.git' } };
const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
function pack(name, metadata, populate) {
  const directory = path.join(output, name);
  fs.mkdirSync(directory);
  populate(directory);
  fs.writeFileSync(path.join(directory, 'package.json'), JSON.stringify({ name, ...common, ...metadata }, null, 2) + '\n');
  const packed = JSON.parse(execFileSync(npm, ['pack', '--ignore-scripts', '--json', '--pack-destination', output], { cwd: directory, encoding: 'utf8' }));
  return packed[0].filename;
}
const archives = [];
for (const input of inputs) {
  const [os, cpu, libc] = platforms[input.target];
  const name = `canisend-cli-${platforms[input.target].join('-')}`;
  archives.push(pack(name, { description: `CanISend native CLI for ${input.target}`, os: [os], cpu: [cpu], ...(libc ? { libc: [libc === 'gnu' ? 'glibc' : libc] } : {}) }, directory => {
    for (const file of ['canisend' + (os === 'win32' ? '.exe' : ''), 'LICENSE', 'THIRD_PARTY_NOTICES.md', 'TYPST-ASSETS-LICENSE', 'TYPST-ASSETS-NOTICE', 'TARGET', 'RELEASE.json']) {
      const source = path.join(input.bundle, file);
      if (!fs.lstatSync(source).isFile()) throw Error(`not a regular file: ${source}`);
      fs.copyFileSync(source, path.join(directory, file));
    }
    fs.chmodSync(path.join(directory, os === 'win32' ? 'canisend.exe' : 'canisend'), 0o755);
  }));
}
archives.push(pack('canisend-cli', {
  description: 'Evidence-bound application preparation CLI', bin: { canisend: 'canisend.cjs' }, engines: { node: '>=22.14' },
  optionalDependencies: Object.fromEntries(Object.values(platforms).map(p => [`canisend-cli-${p.join('-')}`, version])),
}, directory => {
  fs.copyFileSync(fileURLToPath(new URL('canisend.cjs', import.meta.url)), path.join(directory, 'canisend.cjs'));
  fs.copyFileSync(path.join(inputs[0].bundle, 'LICENSE'), path.join(directory, 'LICENSE'));
}));
fs.writeFileSync(path.join(output, 'packages.json'), JSON.stringify({ version, complete: seen.size === Object.keys(platforms).length, archives }, null, 2) + '\n');
