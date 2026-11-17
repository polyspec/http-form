// The recipes of the Makefile that no workflow test reads (AGENTS.md, "Bindings", "CI and release"). The test reads a dry
// run of the recipes (make --dry-run), so no recipe runs.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { test } from 'node:test';

const root = resolve(import.meta.dirname, '../..');
const dryRun = (...goals: string[]): string[] =>
  execFileSync('make', ['--dry-run', '--no-print-directory', ...goals], {
    cwd: root,
    encoding: 'utf8',
    env: { ...process.env, MAKEFLAGS: '', MAKELEVEL: '' },
    stdio: ['ignore', 'pipe', 'pipe'],
  })
    .split('\n')
    .filter((line) => line !== '');

// The release creates and pushes the tag before it builds the archives, so the job check of CI builds them from the
// commit with no tag (make release-candidate-assets); kit reads the version from the change log of HEAD (kit K1286), so
// the recipe names none.
test('the job check of CI builds the archives of the release from the commit with no tag', () => {
  const makefile = readFileSync(resolve(root, 'Makefile'), 'utf8');
  const core = /^CI_CHECK_TARGETS_core := (.*)$/m.exec(makefile)?.[1]?.split(' ') ?? [];
  assert.ok(core.includes('release-candidate-assets'), `the entry core of ci-check runs ${core.join(' ')}`);
  const recipe = dryRun('release-candidate-assets');
  assert.equal(recipe.length, 1, `make release-candidate-assets runs ${String(recipe.length)} lines`);
  assert.match(recipe[0] ?? '', /^RELEASE_ASSETS=var\/release-candidate\/assets +\S+\/node \S+\/release\.mjs candidate-assets$/);
});

// kit builds the archives of a commit with no tag from its release 0.0.120 on (kit K1286) and writes the tracked
// Cargo.lock through its link script from its release 0.0.121 on (kit K1287): package.json names the asset of that
// release and package-lock.json pins it by its integrity.
test('package.json names the release asset of kit 0.0.121 and package-lock.json pins its integrity', () => {
  const asset = 'https://github.com/polyspec/kit/releases/download/v0.0.121/polyspec-kit-0.0.121.tgz';
  const manifest = JSON.parse(readFileSync(resolve(root, 'package.json'), 'utf8')) as { devDependencies?: Record<string, string> };
  assert.equal(manifest.devDependencies?.['@polyspec/kit'], asset);
  const lock = JSON.parse(readFileSync(resolve(root, 'package-lock.json'), 'utf8')) as { packages: Record<string, { devDependencies?: Record<string, string>; resolved?: string; integrity?: string; version?: string }> };
  assert.equal(lock.packages['']?.devDependencies?.['@polyspec/kit'], asset);
  const entry = lock.packages['node_modules/@polyspec/kit'];
  assert.equal(entry?.resolved, asset);
  assert.equal(entry?.version, '0.0.121');
  assert.match(entry?.integrity ?? '', /^sha512-[A-Za-z0-9+/]+=*$/);
});

// The test features of each binding crate are the names of the [features] table of its manifest, which binding scaffold
// writes: a crate built without one has no entry point that a generated test calls. The test reads what make would run,
// so no recipe runs.
const CRATES = [
  { manifest: 'packages/python/Cargo.toml', target: 'test-python', build: /^cd packages\/python && .*maturin build .*--features (\S+)/ },
  { manifest: 'packages/ts/Cargo.toml', target: 'node-package', build: / build .*-p polyspec-http-form-node .*--features (\S+)/ },
  { manifest: 'packages/ts/wasm/Cargo.toml', target: 'node-package', build: / build .*-p polyspec-http-form-wasm .*--features (\S+)/ },
  { manifest: 'packages/php-ext/Cargo.toml', target: 'test-php', build: / build .*-p polyspec-http-form-php-ext .*--features (\S+)/ },
  { manifest: 'packages/go/internal/ffi/Cargo.toml', target: 'test-go', build: / build .*-p polyspec-http-form-go .*--features (\S+)/ },
] as const;

// The names of the [features] table of a manifest, without default.
function featuresOf(manifest: string): string[] {
  const table = /\n\[features\]\n([\s\S]*?)(?:\n\[|$)/.exec(readFileSync(resolve(root, manifest), 'utf8'))?.[1] ?? '';
  return [...table.matchAll(/^([a-z][a-z0-9-]*) = /gm)].map((m) => m[1] ?? '').filter((name) => name !== 'default').sort();
}

test('each binding crate is built with every feature of its manifest', () => {
  const lines = dryRun(...new Set(CRATES.map((crate) => crate.target)));
  for (const crate of CRATES) {
    const declared = featuresOf(crate.manifest);
    assert.ok(declared.includes('panic-test'), `${crate.manifest} declares no feature panic-test`);
    const built = [...new Set(lines.map((line) => crate.build.exec(line)?.[1]).filter((found) => found !== undefined))];
    assert.equal(built.length, 1, `the recipes build ${crate.manifest} with ${String(built.length)} lists of features: ${built.join(' ')}`);
    assert.deepEqual(built[0]?.split(',').sort(), declared, `${crate.manifest} is built with the features ${built[0] ?? ''}, its manifest declares ${declared.join(',')}`);
  }
});

// binding writes each generated Go test under the build tag polyspec_<library>_<feature>, the name of the library with
// `_` for `-`, so go test builds the generated tests of a feature only with the tag of that feature of the Go crate.
test('make test-go gives go test the build tag polyspec_http_form_<feature> of each feature of the Go crate', () => {
  const expected = featuresOf('packages/go/internal/ffi/Cargo.toml').map((name) => `polyspec_http_form_${name.replaceAll('-', '_')}`).sort();
  const goTest = dryRun('test-go').filter((line) => / test -count=1 /.test(line));
  assert.equal(goTest.length, 1, `go test runs ${String(goTest.length)} times`);
  const tags = /GOFLAGS=-tags=(\S+)/.exec(goTest[0] ?? '')?.[1]?.split(',').sort() ?? [];
  assert.deepEqual(tags, expected, `go test runs with the tags ${tags.join(',')}`);
});

// A list of features that a recipe names itself is a second copy of the manifest, which a new feature of binding leaves
// behind: every recipe of a binding crate takes its features from a variable that reads a manifest. The recipes of the
// core (-p $(CORE)) name the feature of the core that they build: wasm-core builds it as the WASM crate does, without
// the default features and with fault-test (binding GN-4a, GN-5), which no table of a manifest lists.
test('no recipe of a binding crate names a feature itself', () => {
  const literal = readFileSync(resolve(root, 'Makefile'), 'utf8').split('\n').filter((line) => (/--features (?!\$\()/.test(line) && !line.includes('-p $(CORE) ')) || /^[A-Z_]*FEATURES :?= *[a-z]/.test(line));
  assert.deepEqual(literal, [], 'a recipe or a variable of the Makefile names features itself');
});

// Cargo.lock is written by cargo: after a change of the tag of a crate in a Cargo.toml, make cargo-lock runs cargo
// update for the crates of CRATES through the link script of kit in its form --tracked-lock (kit K1287), which runs
// cargo as given, with no patch and no copy of the lock, in a checkout with or without a link declaration. The recipe
// downloads, so it runs through $(ONLINE), and it starts no cargo without CRATES.
test('make cargo-lock writes Cargo.lock for the crates of CRATES through the tracked-lock form of the link script', () => {
  const recipe = dryRun('cargo-lock', 'CRATES=polyspec-one polyspec-two', 'ONLINE=ONLINE-MARK');
  assert.match(recipe.at(-1) ?? '', /^ONLINE-MARK \/\S+\/node \/\S+\/link\.mjs cargo --tracked-lock \/\S+ update -p polyspec-one -p polyspec-two$/, recipe.join('\n'));
  const empty = spawnSync('make', ['--no-print-directory', 'cargo-lock', 'NODE=false', 'CARGO=false'], { cwd: root, encoding: 'utf8', env: { ...process.env, MAKEFLAGS: '', MAKELEVEL: '' } });
  assert.equal(empty.status, 2, empty.stderr);
  assert.match(empty.stderr, /cargo-lock: CRATES is empty/);
});
