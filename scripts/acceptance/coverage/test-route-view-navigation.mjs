import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(coverageRoot, '../../..');
const validatorPath = path.join(coverageRoot, 'validate-route-view-navigation.mjs');
const evidencePath = path.join(repoRoot, 'docs/testing/PORTAL_ROUTE_VIEW_NAVIGATION_EVIDENCE.json');
const fixtureRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'oteryn-route-view-navigation-'));
const fixturePath = path.join(fixtureRoot, 'invalid-route-view-navigation-evidence.json');

try {
  const fixture = JSON.parse(fs.readFileSync(evidencePath, 'utf8'));
  assert.ok(Array.isArray(fixture.unreferenced_page_classifications));
  assert.ok(fixture.unreferenced_page_classifications.length >= 3);

  const [removed] = fixture.unreferenced_page_classifications.splice(0, 1);
  const duplicate = fixture.unreferenced_page_classifications[0];
  fixture.unreferenced_page_classifications[1] = {
    ...fixture.unreferenced_page_classifications[1],
    classification: 'unbounded_exception',
  };
  fixture.unreferenced_page_classifications.push({ ...duplicate });
  fixture.unreferenced_page_classifications.push({
    path: 'resources/views/does-not-exist.blade.php',
    classification: 'framework_convention',
    rationale: 'Negative fixture proving that stale classifications fail closed.',
    evidence: ['scripts/acceptance/coverage/test-route-view-navigation.mjs'],
  });

  fs.writeFileSync(fixturePath, `${JSON.stringify(fixture, null, 2)}\n`);

  const result = spawnSync(process.execPath, [validatorPath, `--evidence=${fixturePath}`], {
    cwd: repoRoot,
    encoding: 'utf8',
    env: process.env,
  });

  assert.notEqual(result.status, 0, 'invalid route/view/navigation evidence must fail');
  assert.match(result.stdout, new RegExp(`Unclassified page-like Blade view ${removed.path.replaceAll('/', '\\/')}`));
  assert.match(result.stdout, /Unsupported view classification unbounded_exception/u);
  assert.match(result.stdout, new RegExp(`Duplicate view classification ${duplicate.path.replaceAll('/', '\\/')}`));
  assert.match(result.stdout, /Stale view classification resources\/views\/does-not-exist\.blade\.php/u);

  process.stdout.write('route/view/navigation negative fixture: PASS\n');
} finally {
  fs.rmSync(fixtureRoot, { recursive: true, force: true });
}
