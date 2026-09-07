import { test, expect } from '@playwright/test';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { repoRoot, testedSha, runBinary } from './helpers.mjs';

// Temporary, owner-authorized tracked-source transport for the offline editor.
// Exports Git trees only: never runtime files, environment, sessions or databases.
// Remove before readiness. It does not alter the application or any quality gate.
test('@smoke @portal-review recover exact tracked implementation sources', () => {
  const output = resolve(repoRoot, 'artifacts/acceptance/portal-source-recovery');
  mkdirSync(output, { recursive: true });
  const head = runBinary('git', ['rev-parse', 'HEAD']);
  expect(head).toBe(testedSha);
  const base = '3557085c20512d25576d8884cc54471665784b00';
  for (const [name, ref] of [['candidate', head], ['main', base]]) {
    runBinary('git', ['archive', '--format=tar', `--prefix=${name}/`, `--output=${resolve(output, `${name}.tar`)}`, ref]);
  }
  writeFileSync(resolve(output, 'provenance.json'), JSON.stringify({ head, base, source: 'git archive; tracked files only' }, null, 2));
});
