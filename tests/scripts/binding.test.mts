// binding is a released dependency (AGENTS.md, "Bindings"): the crate polyspec-binding-core of the core and of every
// binding crate is the tag of a release, Cargo.lock pins the commit of that tag, and the tool @polyspec/binding is the
// archive of the same release, which package.json names by the URL of its release asset and package-lock.json pins
// by its integrity. The generator and binding_core are then one version (G-6), and binding check takes the tag of
// Cargo.lock as its own (CK-20). The recipes run the installed tool by its path, and only make install-binding
// downloads: cargo fetch --locked, because the tool reads the crates with cargo metadata --locked --offline.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { test } from 'node:test';

const root = resolve(import.meta.dirname, '../..');
const read = (file: string): string => readFileSync(join(root, file), 'utf8');
// The one source of the version of binding is the asset that package.json names: every other place must name that release.
const ASSET_OF_RELEASE = /^https:\/\/github\.com\/polyspec\/binding\/releases\/download\/v(\d+\.\d+\.\d+)\/polyspec-binding-\1\.tgz$/;
const declared = (JSON.parse(read('package.json')) as { devDependencies?: Record<string, string> }).devDependencies?.['@polyspec/binding'] ?? '';
const VERSION = ASSET_OF_RELEASE.exec(declared)?.[1] ?? '';
assert.ok(VERSION !== '', `package.json names no release asset of binding: ${declared}`);
const ASSET = declared;
// The release of http-media-type that the core names (its tag in packages/rust/Cargo.toml) is the one whose Cargo.lock pins the same binding.
const MEDIA_TYPE = /polyspec-http-media-type = \{[^}]*tag = "v(\d+\.\d+\.\d+)"/.exec(read('packages/rust/Cargo.toml'))?.[1] ?? '';
assert.ok(MEDIA_TYPE !== '', 'packages/rust/Cargo.toml names no release of http-media-type');
const dryRun = (...goals: string[]): string[] =>
  execFileSync('make', ['--dry-run', '--no-print-directory', ...goals], {
    cwd: root,
    encoding: 'utf8',
    env: { ...process.env, MAKEFLAGS: '', MAKELEVEL: '' },
    stdio: ['ignore', 'pipe', 'pipe'],
  })
    .split('\n')
    .filter((line) => line !== '');

test('every Cargo manifest names polyspec-binding-core by the tag of the release', () => {
  const manifests = execFileSync('git', ['ls-files', '*Cargo.toml'], { cwd: root, encoding: 'utf8' }).split('\n').filter((file) => file !== '');
  const lines = manifests.flatMap((file) => read(file).split('\n').filter((line) => /^polyspec-binding-core\s*=/.test(line)).map((line) => ({ file, line })));
  assert.ok(lines.length > 0, 'no manifest names polyspec-binding-core');
  for (const { file, line } of lines) {
    assert.match(line, new RegExp(`tag = "v${VERSION.replaceAll('.', '\\.')}"`), `${file}: ${line}`);
    assert.doesNotMatch(line, /branch\s*=|rev\s*=/, `${file}: ${line}`);
  }
});

test('Cargo.lock pins polyspec-binding-core at the tag of the release', () => {
  const sources = read('Cargo.lock').split('\n').filter((line) => line.startsWith('source = "git+https://github.com/polyspec/binding.git'));
  assert.ok(sources.length > 0, 'Cargo.lock has no source of binding');
  for (const source of sources) assert.match(source, new RegExp(`\\?tag=v${VERSION.replaceAll('.', '\\.')}#[0-9a-f]{40}"$`), source);
});

test('package.json names the release asset of the tool and package-lock.json pins its integrity', () => {
  const manifest = JSON.parse(read('package.json')) as { devDependencies?: Record<string, string> };
  assert.equal(manifest.devDependencies?.['@polyspec/binding'], ASSET);
  const lock = JSON.parse(read('package-lock.json')) as { packages: Record<string, { resolved?: string; integrity?: string; version?: string }> };
  const entry = lock.packages['node_modules/@polyspec/binding'];
  assert.equal(entry?.resolved, ASSET);
  assert.equal(entry?.version, VERSION);
  assert.match(entry?.integrity ?? '', /^sha512-[A-Za-z0-9+/]+=*$/);
});

test('the recipes run the installed tool, and make install-binding fetches the crates of Cargo.lock', () => {
  const check = dryRun('bindings-check');
  assert.ok(check.some((line) => line.endsWith('node_modules/@polyspec/binding/dist/cli.js check packages/rust/interface.json --cargo "cargo"')), check.join('\n'));
  // binding takes the cargo command of the repository for every command that runs Cargo (binding GN-1d).
  for (const target of ['php', 'python', 'node', 'go', 'wasm']) {
    const generate = dryRun(`bindings-${target}`);
    assert.ok(generate.some((line) => line.endsWith(`node_modules/@polyspec/binding/dist/cli.js generate --root . --target ${target} --cargo "cargo"`)), generate.join('\n'));
  }
  const install = dryRun('install-binding');
  assert.ok(install.some((line) => / fetch --locked( |$)/.test(line)), install.join('\n'));
});

// Cargo holds two releases of one Git dependency as two crates, so the core names the release of http-media-type that
// depends on the same binding, and Cargo.lock pins one polyspec-binding-core (binding CK-20).
test('the core names the release of http-media-type on the same binding, and Cargo.lock pins one polyspec-binding-core', () => {
  const lines = read('packages/rust/Cargo.toml').split('\n').filter((line) => /^polyspec-http-media-type\s*=/.test(line));
  assert.equal(lines.length, 1, 'packages/rust/Cargo.toml names polyspec-http-media-type once');
  assert.match(lines[0] ?? '', new RegExp(`tag = "v${MEDIA_TYPE.replaceAll('.', '\\.')}"`), lines[0]);
  const lock = read('Cargo.lock');
  assert.equal(lock.split('name = "polyspec-binding-core"').length - 1, 1, 'Cargo.lock holds one polyspec-binding-core');
  const sources = lock.split('\n').filter((line) => line.startsWith('source = "git+https://github.com/polyspec/http-media-type.git'));
  assert.equal(sources.length, 1, 'Cargo.lock holds one source of http-media-type');
  assert.match(sources[0] ?? '', new RegExp(`\\?tag=v${MEDIA_TYPE.replaceAll('.', '\\.')}#[0-9a-f]{40}"$`), sources[0]);
});

// The tracked binding packages, go.mod and go.sum among them, are what binding scaffold writes (binding L-2), and binding
// check compares them (CK-19), so a change of the pin of binding writes them again with make bindings-scaffold. The
// recipe downloads, because go.sum holds the sums that go mod download gives (binding GN-15l), so it runs through
// $(ONLINE).
test('make bindings-scaffold writes the tracked binding packages with the installed tool, through ONLINE', () => {
  const recipe = dryRun('bindings-scaffold', 'ONLINE=ONLINE-MARK');
  assert.equal(recipe.length, 1, recipe.join('\n'));
  assert.match(recipe[0] ?? '', /^ONLINE-MARK \/\S+\/node \/\S+\/node_modules\/@polyspec\/binding\/dist\/cli\.js scaffold --root \. --cargo "cargo"$/);
});
