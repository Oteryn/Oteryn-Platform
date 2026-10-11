import fs from 'node:fs';
import path from 'node:path';
import { test, expect } from '@playwright/test';
import {
  allowExpectedHttpFailure,
  attachDiagnostics,
  installDiagnostics,
  repoRoot,
} from './helpers.mjs';

const stressEvidenceMarker = '@portal-community-stress long values multi-page results internal 500 containment and recovery';
const communityCoveragePath = path.join(
  repoRoot,
  'scripts/acceptance/coverage/surfaces/community-data-completeness.json',
);
const communityCoverage = JSON.parse(fs.readFileSync(communityCoveragePath, 'utf8'));
const stressEvidence = communityCoverage.state_evidence_extensions?.find(
  (entry) => entry.surface_id === 'public.game-data' && entry.issue === 350,
);
const requiredStressStates = [
  'very-long-values',
  'large-result-set-pagination',
  'internal-error-500-contained',
  'internal-error-restored',
];

if (!stressEvidence) {
  throw new Error('Missing Issue #350 public.game-data state evidence extension.');
}
if (JSON.stringify(stressEvidence.states?.map((state) => state.id)) !== JSON.stringify(requiredStressStates)) {
  throw new Error('Issue #350 state evidence extension does not define the exact required state order.');
}
if (stressEvidence.evidence?.file !== 'scripts/acceptance/tests/community-data-acceptance.spec.mjs') {
  throw new Error('Issue #350 state evidence points at an unexpected evidence file.');
}
if (stressEvidence.evidence?.marker !== stressEvidenceMarker) {
  throw new Error('Issue #350 state evidence marker does not match the executable browser marker.');
}
if (stressEvidence.retries !== 0) {
  throw new Error('Issue #350 state evidence must remain zero retry.');
}

const currentSpec = fs.readFileSync(path.join(repoRoot, stressEvidence.evidence.file), 'utf8');
if (!currentSpec.includes(stressEvidenceMarker)) {
  throw new Error('Issue #350 executable browser marker is missing from the referenced evidence file.');
}

test.setTimeout(180_000);
test.describe.configure({ retries: 0, mode: 'serial' });

// Game-world community data has no native source yet. Until native fixtures return, every
// community route must fail closed to the truthful localized unavailable state (503),
// never an empty ranking, a stack trace, a SQL fragment or a database name.
const communityRoutes = [
  '/highscores', '/deaths', '/guilds', '/online', '/servers',
  '/characters/Acceptance%20Hero', '/guilds/Acceptance%20Guild',
];

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

async function expectNoHorizontalOverflow(page) {
  const dimensions = await page.evaluate(() => ({
    documentWidth: document.documentElement.scrollWidth,
    viewportWidth: document.documentElement.clientWidth,
  }));
  expect(dimensions.documentWidth).toBeLessThanOrEqual(dimensions.viewportWidth + 1);
}

async function expectUnavailable(page, locale, route) {
  const url = `/${locale}${route}`;
  allowExpectedHttpFailure(page.__acceptanceDiagnostics, { status: 503, pathname: url.split('?')[0] });
  const response = await page.goto(url, { waitUntil: 'domcontentloaded' });
  expect(response?.status(), url).toBe(503);
  await expect(page.locator('.community-unavailable h1')).toBeVisible();
  const body = await page.locator('body').innerText();
  for (const forbidden of ['SQLSTATE', 'Stack trace', 'Illuminate\\', process.env.DB_DATABASE, process.env.DB_PASSWORD]
    .filter((value) => typeof value === 'string' && value !== '')) {
    expect(body).not.toContain(forbidden);
  }
  await expectNoHorizontalOverflow(page);
}

test('@portal-community complete rankings, privacy-aware profile, deaths, guild search, localization, resilience and responsive lifecycle', async ({ page }) => {
  for (const locale of ['en', 'pl']) {
    for (const route of communityRoutes) await expectUnavailable(page, locale, route);
  }
});

test('@portal-community-stress long values, multi-page results, internal 500 containment and recovery', async ({ page }) => {
  // The stress states stay unavailable with the data source; the contained, non-debug failure surface is what is proven.
  for (const route of ['/highscores?page=2', '/deaths?page=2', '/guilds?search=' + 'x'.repeat(120)]) {
    await expectUnavailable(page, 'en', route);
  }
});
