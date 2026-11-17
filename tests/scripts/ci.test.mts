// The tools, the workflows and the release configuration of the repository (AGENTS.md, "Tools" and "CI and release"):
// every tool of a recipe is pinned in a tracked file and installed into var/tools, no recipe takes a tool of the
// machine, and a workflow runs pinned actions and one make target per command on ubuntu-26.04-arm without a time limit.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
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

// The runners, the pins, the time limits and the run steps of every workflow are checked by
// tests/scripts/workflows.test.mts; this test checks the events and the last job of ci.yml.
test('ci.yml runs on the push of main and on a manual run and ends with ci-passed', () => {
  const ci = read('.github/workflows/ci.yml');
  assert.match(ci, /^\s*branches: \[main\]$/m, 'ci.yml runs on the push of main');
  assert.match(ci, /workflow_dispatch/, 'ci.yml runs on a manual run');
  assert.match(ci, /^\s*ci-passed:$/m, 'ci.yml ends with the job ci-passed');
  assert.deepEqual(readdirSync(join(root, '.github/workflows')).sort(), ['assets.yml', 'ci.yml', 'release.yml'], 'the workflows are assets.yml, ci.yml and release.yml');
});

test('the release configuration names the repository and a version 0.0.x', () => {
  const release = JSON.parse(read('config/release.json')) as { repositoryUrl?: string; manifests?: Record<string, string>; notReleased?: Record<string, string>; goModules?: Record<string, string> };
  assert.equal(release.repositoryUrl, 'https://github.com/polyspec/http-form');
  assert.equal(release.manifests?.['packages/rust/Cargo.toml'], 'git-tag');
  // make release-go-module tags the Go module below the root with packages/go/vX.Y.Z at the commit that carries its
  // generated files and libraries (binding GN-19f), so no tag of kit releases it.
  assert.ok(/^module \S+$/m.test(read('packages/go/go.mod')), 'packages/go/go.mod names no module');
  assert.deepEqual(release.goModules, {}, 'goModules names no module');
  assert.match(release.notReleased?.['packages/go/go.mod'] ?? '', /release-go-module/, 'notReleased names how the Go module is released');
  assert.match(/^\[workspace\.package\][^[]*?^version = "([^"]+)"$/ms.exec(read('Cargo.toml'))?.[1] ?? '', /^0\.0\.\d+$/, 'the version is 0.0.x');
});

test('make bootstrap installs the lock with the npm of devEngines, outside the checkout, through ONLINE (T1.21)', () => {
  const npm = (JSON.parse(read('package.json')) as { devEngines?: { packageManager?: { version?: string } } }).devEngines?.packageManager?.version;
  assert.match(npm ?? '', EXACT, 'devEngines names an exact npm');
  const lines = execFileSync('make', ['--dry-run', '--no-print-directory', 'bootstrap', 'ONLINE=ONLINE-MARK'], {
    cwd: root,
    encoding: 'utf8',
    env: { ...process.env, MAKEFLAGS: '', MAKELEVEL: '' },
    stdio: ['ignore', 'pipe', 'ignore'],
  }).split('\n').filter((line) => line !== '');
  const recipe = lines.join('\n');
  assert.match(recipe, /devEngines\.packageManager\.version/, `the recipe does not read devEngines:\n${recipe}`);
  assert.match(recipe, /exec --yes --package="npm@\$version" -- npm --prefix "\$root" ci --ignore-scripts/, `the recipe does not install the lock:\n${recipe}`);
  assert.match(recipe, /cd "\$\{TMPDIR:-\/tmp\}"/, `the recipe runs inside the checkout:\n${recipe}`);
  assert.match(recipe, /ONLINE-MARK /, `the download does not run through $(ONLINE):\n${recipe}`);
});

test('the release publishes the form cases with their digests through the release tooling (T1.25, kit K1267)', async () => {
  const cases = ['contracts/fields.json', 'contracts/forms.json'];
  const release = JSON.parse(read('config/release.json')) as { files?: string[] };
  assert.deepEqual(release.files, cases, 'config/release.json: files does not name the two case files');
  const kit = (await import(join(root, 'node_modules/@polyspec/kit/dist/release.mjs'))) as { loadConfig: (dir: string) => { files?: string[] } };
  assert.deepEqual(kit.loadConfig(root).files, cases, 'the release tooling of the installed kit does not read files');
});

// The release-wheel recipe runs the maturin of MATURIN, which packages/release.mk sets with ?= to the shim of mise.toml; a
// := of the Makefile before its include would keep the pip requirement of the test of python in its place (GN-19c).
test('the Makefile leaves MATURIN to the maturin of mise.toml, which release-wheel runs', () => {
  assert.doesNotMatch(read('Makefile'), /^MATURIN\s*:=/m, 'the Makefile assigns MATURIN with := ahead of packages/release.mk');
});

// The tools that the release-wheel recipe installs with mise are pinned in mise.toml and locked by mise.lock for every
// platform that toolchain-lock writes, so no release job meets a tool that its lock does not name (GN-19c). The recipe is
// generated (make release-recipes) and not tracked, so the tools it installs are named here.
test('every tool of the release-wheel recipe is pinned in mise.toml and locked on every platform of toolchain-lock', () => {
  const installed = ['github:PyO3/maturin'];
  const platforms = (/^TOOLCHAIN_LOCK_PLATFORMS := (.+)$/m.exec(read('Makefile'))?.[1] ?? '').split(',');
  assert.ok(platforms.length > 0 && platforms[0] !== '', 'the Makefile names no platform of toolchain-lock');
  const mise = read('mise.toml');
  const lock = read('mise.lock');
  for (const tool of installed) {
    assert.ok(mise.includes(`"${tool}" = "`), `mise.toml does not pin ${tool}`);
    for (const platform of platforms) {
      assert.ok(lock.includes(`[tools."${tool}"."platforms.${platform}"]`), `mise.lock does not lock ${tool} for ${platform}`);
    }
  }
});
