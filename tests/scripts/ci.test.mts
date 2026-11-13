// The tools, the workflows and the release configuration of the repository (AGENTS.md, "Tools" and "CI and release"):
// every tool of a recipe is pinned in a tracked file and installed into var/tools, no recipe takes a tool of the
// machine, and a workflow runs pinned actions and one make target per command on ubuntu-26.04-arm without a time limit.
import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { test } from 'node:test';

const root = resolve(import.meta.dirname, '../..');
const read = (path: string): string => {
  assert.ok(existsSync(join(root, path)), `${path} is missing`);
  return readFileSync(join(root, path), 'utf8');
};
const EXACT = /^\d+\.\d+\.\d+$/;
const SHA256 = /^[0-9a-f]{64}$/;

test('every tool is pinned by its exact release in a tracked file', () => {
  const mise = read('mise.toml');
  const lock = read('mise.lock');
  const tools = [...mise.matchAll(/^"?([A-Za-z0-9:/_-]+)"? = (?:"([^"]+)"|\{ version = "([^"]+)")/gm)]
    .map((match) => ({ name: match[1] ?? '', version: match[2] ?? match[3] ?? '' }))
    .filter((tool) => !['locked', 'auto_install'].includes(tool.name));
  for (const name of ['node', 'npm:npm', 'go', 'python', 'http:composer', 'github:wasm-bindgen/wasm-bindgen']) {
    const tool = tools.find((candidate) => candidate.name === name);
    assert.ok(tool !== undefined, `mise.toml does not name ${name}`);
    assert.match(tool.version, EXACT, `mise.toml: ${name} is not an exact release`);
    assert.ok(lock.includes(`version = "${tool.version}"`), `mise.lock does not lock ${name} ${tool.version}`);
  }
  assert.match(mise, /^locked = true$/m, 'mise.toml: mise runs only what mise.lock pins');
  assert.match(mise, /^auto_install = false$/m, 'mise.toml: mise installs nothing on first use');

  const toolchain = JSON.parse(read('config/toolchain.json')) as { mise?: { version?: string; sha256?: Record<string, string> } };
  assert.match(toolchain.mise?.version ?? '', EXACT, 'config/toolchain.json: the release of mise');
  for (const platform of ['darwin-arm64', 'linux-x64', 'linux-arm64']) {
    assert.match(toolchain.mise?.sha256?.[platform] ?? '', SHA256, `config/toolchain.json: the sha256 of mise for ${platform}`);
  }

  assert.match(/^channel = "([^"]+)"$/m.exec(read('rust-toolchain.toml'))?.[1] ?? '', EXACT, 'rust-toolchain.toml: the channel');
  assert.match(read('.php-version').trim(), /^\d+\.\d+$/, '.php-version: the minor of PHP');
  const npm = (JSON.parse(read('package.json')) as { devEngines?: { packageManager?: { name?: string; version?: string } } }).devEngines?.packageManager;
  assert.equal(npm?.name, 'npm');
  assert.equal(npm?.version, tools.find((tool) => tool.name === 'npm:npm')?.version, 'devEngines and mise.toml name one npm');
  assert.ok(existsSync(join(root, 'composer.lock')), 'composer.lock pins PHPUnit');
});

test('no recipe takes a tool of the machine', () => {
  const makefile = read('Makefile');
  assert.match(makefile, /^KIT_TOOLS := \$\(CURDIR\)\/var\/tools$/m, 'the tools are those of var/tools');
  assert.match(makefile, /^export PATH := \$\(KIT_TOOLS\)\/bin:\$\(KIT_TOOLS\)\/mise\/shims:\$\(PATH\)$/m, 'var/tools is first on PATH');
  assert.match(makefile, /^export RUSTUP_AUTO_INSTALL := 0$/m, 'rustup installs no toolchain on first use');
  const found = makefile.split('\n').filter((line) => /^(PHP|PHP_CONFIG|PHPUNIT|MATURIN|GO|GOFMT|WASM_BINDGEN|NODE|NPM|BINDING)\s*[?:]?=/.test(line))
    .filter((line) => /command -v/.test(line) || /=\s*(php|php-config|phpunit|maturin|go|gofmt|wasm-bindgen|node|npm)?\s*$/.test(line));
  assert.deepEqual(found, [], 'a tool variable names a tool of the machine or nothing');
  for (const target of ['bootstrap', 'install', 'install-tools', 'install-packages', 'install-php-tools', 'toolchain-lock']) {
    assert.match(makefile, new RegExp(`^${target}:`, 'm'), `the Makefile has no target ${target}`);
  }
});

test('a workflow runs pinned actions and one make target per command', () => {
  const makefile = read('Makefile');
  const targets = new Set([...makefile.matchAll(/^([a-z][a-z0-9-]*):/gm)].map((match) => match[1] ?? ''));
  const files = ['.github/workflows/ci.yml', '.github/workflows/release.yml', '.github/actions/bootstrap/action.yml'];
  let runs = 0;
  let uses = 0;
  let jobs = 0;
  for (const file of files) {
    const lines = read(file).split('\n').filter((line) => !line.trim().startsWith('#'));
    assert.ok(!lines.some((line) => /timeout-minutes/.test(line)), `${file} sets a time limit`);
    // The steps are below `jobs:` of a workflow and `runs:` of an action; a key `run` above them is an input of the workflow.
    const steps = lines.slice(lines.findIndex((line) => /^(jobs|runs):/.test(line)));
    for (const line of steps) {
      const runsOn = /^\s*runs-on:\s*(.+)$/.exec(line)?.[1];
      if (runsOn !== undefined) {
        jobs += 1;
        assert.equal(runsOn.trim(), 'ubuntu-26.04-arm', `${file}: a job runs on ${runsOn}`);
      }
      const action = /^\s*-?\s*uses:\s*(\S+)/.exec(line)?.[1];
      if (action !== undefined) {
        uses += 1;
        assert.ok(action.startsWith('./') || /@[0-9a-f]{40}$/.test(action), `${file}: ${action} is not pinned by a commit`);
      }
      const command = /^\s*-?\s*run:\s*(.*)$/.exec(line)?.[1];
      if (command !== undefined) {
        runs += 1;
        const target = /^make ([a-z][a-z0-9-]*)(?: |$)/.exec(command)?.[1];
        assert.ok(target !== undefined, `${file}: the command "${command}" is not one make target`);
        assert.ok(targets.has(target), `${file}: the Makefile has no target ${target}`);
      }
    }
  }
  assert.ok(jobs >= 3 && uses > 0 && runs > 0, `the workflows were read: ${jobs} jobs, ${uses} actions, ${runs} commands`);
  const ci = read('.github/workflows/ci.yml');
  assert.match(ci, /^\s*branches: \[main\]$/m, 'ci.yml runs on the push of main');
  assert.match(ci, /workflow_dispatch/, 'ci.yml runs on a manual run');
  assert.match(ci, /^\s*ci-passed:$/m, 'ci.yml ends with the job ci-passed');
  assert.ok(readdirSync(join(root, '.github/workflows')).length === 2, 'the workflows are ci.yml and release.yml');
});

test('the release configuration names the repository and a version 0.0.x', () => {
  const release = JSON.parse(read('config/release.json')) as { repositoryUrl?: string; manifests?: Record<string, string> };
  assert.equal(release.repositoryUrl, 'https://github.com/polyspec/http-form');
  assert.equal(release.manifests?.['packages/rust/Cargo.toml'], 'git-tag');
  assert.match(/^\[workspace\.package\][^[]*?^version = "([^"]+)"$/ms.exec(read('Cargo.toml'))?.[1] ?? '', /^0\.0\.\d+$/, 'the version is 0.0.x');
});
