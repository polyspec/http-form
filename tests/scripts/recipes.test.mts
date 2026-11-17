// The recipes of the Makefile that no workflow test reads (AGENTS.md, "Bindings", "CI and release"). The test reads a dry
// run of the recipes (make --dry-run), so no recipe runs.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
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

// kit builds the archives of a commit with no tag from its release 0.0.120 on (kit K1286): package.json names the asset
// of that release and package-lock.json pins it by its integrity.
test('package.json names the release asset of kit 0.0.120 and package-lock.json pins its integrity', () => {
  const asset = 'https://github.com/polyspec/kit/releases/download/v0.0.120/polyspec-kit-0.0.120.tgz';
  const manifest = JSON.parse(readFileSync(resolve(root, 'package.json'), 'utf8')) as { devDependencies?: Record<string, string> };
  assert.equal(manifest.devDependencies?.['@polyspec/kit'], asset);
  const lock = JSON.parse(readFileSync(resolve(root, 'package-lock.json'), 'utf8')) as { packages: Record<string, { devDependencies?: Record<string, string>; resolved?: string; integrity?: string; version?: string }> };
  assert.equal(lock.packages['']?.devDependencies?.['@polyspec/kit'], asset);
  const entry = lock.packages['node_modules/@polyspec/kit'];
  assert.equal(entry?.resolved, asset);
  assert.equal(entry?.version, '0.0.120');
  assert.match(entry?.integrity ?? '', /^sha512-[A-Za-z0-9+/]+=*$/);
});
