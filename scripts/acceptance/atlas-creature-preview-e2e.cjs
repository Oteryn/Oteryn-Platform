'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const { chromium } = require('/opt/oteryn-playwright/node_modules/@playwright/test');

const preview = process.env.PREVIEW_URL;
const expectedRevision = process.env.ATLAS_REV;
const expectedDigest = process.env.CREATURE_SEMANTIC_DIGEST;
const targets = JSON.parse(fs.readFileSync('/targets.json', 'utf8'));
const evidenceDir = '/evidence';
fs.mkdirSync(evidenceDir, { recursive: true });

assert.match(preview ?? '', /^http:\/\/192\.168\.1\.2:8097$/);
assert.match(expectedRevision ?? '', /^[0-9a-f]{40}$/);
assert.match(expectedDigest ?? '', /^sha256:[0-9a-f]{64}$/);
assert.equal(targets.npc.kind, 'npc');
assert.equal(targets.monster.kind, 'monster');

function escapeRegex(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

async function diagnostic(page) {
  return page.evaluate(() => globalThis.__OTERYN_ATLAS_CREATURES__ ?? null);
}

async function waitReady(page, { selectedId = null, npc = true, monster = true } = {}) {
  await page.waitForFunction(
    ({ digest, selected, wantNpc, wantMonster }) => {
      const value = globalThis.__OTERYN_ATLAS_CREATURES__;
      if (!value || value.status !== 'PASS') return false;
      if (value.sourceSemanticDigest !== digest) return false;
      if (value.cacheChunks > 96 || value.drawnRecords < 1) return false;
      if (value.enabled?.npc !== wantNpc || value.enabled?.monster !== wantMonster) return false;
      if (selected && (value.selectedRecordId !== selected || value.selectedVisible !== true)) return false;
      return true;
    },
    { digest: expectedDigest, selected: selectedId, wantNpc: npc, wantMonster: monster },
    { timeout: 120_000 },
  );
  const value = await diagnostic(page);
  assert.equal(value.status, 'PASS');
  assert.equal(value.sourceSemanticDigest, expectedDigest);
  assert.equal(value.totalRecords, 88633);
  assert.equal(value.totalChunks, 5746);
  assert.equal(value.searchRecords, 1945);
  assert.ok(value.drawnRecords > 0);
  assert.ok(value.cacheChunks <= 96);
  return value;
}

function targetUrl(target, creatures = 'npc,monster') {
  const params = new URLSearchParams({
    x: String(target.position.x),
    y: String(target.position.y),
    floor: String(target.position.floor),
    zoom: '2',
    mode: 'minimap',
    perf: 'reference',
    creatures,
    animation: 'off',
  });
  return `${preview}/web/fullworld.html?${params}`;
}

async function assertRevisionResponse(page) {
  const response = await page.request.get(`${preview}/web/fullworld.html`);
  assert.equal(response.status(), 200);
  assert.equal(response.headers()['x-oteryn-atlas-revision'], expectedRevision);
}

async function searchAndSelect(page, target, kindText) {
  const search = page.locator('#creature-search');
  await search.fill(target.label);
  const pattern = new RegExp(`${escapeRegex(target.label)}.*${kindText}`, 'i');
  const result = page.locator('#creature-results button').filter({ hasText: pattern }).first();
  await result.waitFor({ state: 'visible', timeout: 30_000 });
  await Promise.all([
    page.waitForNavigation({ waitUntil: 'domcontentloaded', timeout: 60_000 }),
    result.click(),
  ]);
}

async function runDesktop(browser) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  await page.goto(targetUrl(targets.npc), { waitUntil: 'domcontentloaded', timeout: 60_000 });
  await assertRevisionResponse(page);
  const initial = await waitReady(page);
  assert.ok(initial.visibleRecords > 0);
  assert.equal(await page.locator('input[data-creature-kind="npc"]').isChecked(), true);
  assert.equal(await page.locator('input[data-creature-kind="monster"]').isChecked(), true);
  await page.screenshot({ path: `${evidenceDir}/desktop-initial.png`, fullPage: true });

  await searchAndSelect(page, targets.npc, 'npc');
  await waitReady(page, { selectedId: targets.npc.record_id });
  const inspector = await page.locator('#creature-inspector').innerText();
  assert.match(inspector, new RegExp(escapeRegex(targets.npc.label), 'i'));
  assert.ok(inspector.includes(targets.npc.record_id));
  assert.ok(inspector.includes(expectedDigest));
  assert.match(inspector, /Static marker fallback/i);
  const selectedParams = new URL(page.url()).searchParams;
  assert.equal(selectedParams.get('creature'), targets.npc.record_id);
  assert.equal(selectedParams.get('creatures'), 'monster,npc');

  const monsterToggle = page.locator('input[data-creature-kind="monster"]');
  await monsterToggle.uncheck();
  await waitReady(page, { selectedId: targets.npc.record_id, npc: true, monster: false });
  assert.equal(new URL(page.url()).searchParams.get('creatures'), 'npc');
  await page.screenshot({ path: `${evidenceDir}/desktop-npc-only.png`, fullPage: true });
  await context.close();
  return { initialVisibleRecords: initial.visibleRecords, selected: targets.npc.record_id };
}

async function runMobile(browser) {
  const context = await browser.newContext({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
  const page = await context.newPage();
  await page.goto(targetUrl(targets.monster), { waitUntil: 'domcontentloaded', timeout: 60_000 });
  await assertRevisionResponse(page);
  const initial = await waitReady(page);
  assert.ok(initial.visibleRecords > 0);

  await page.locator('#mobile-controls-toggle').click();
  const npcToggle = page.locator('input[data-creature-kind="npc"]');
  const monsterToggle = page.locator('input[data-creature-kind="monster"]');
  await npcToggle.waitFor({ state: 'visible', timeout: 30_000 });
  assert.equal(await npcToggle.isChecked(), true);
  assert.equal(await monsterToggle.isChecked(), true);
  await npcToggle.uncheck();
  await waitReady(page, { npc: false, monster: true });
  assert.equal(new URL(page.url()).searchParams.get('creatures'), 'monster');

  await searchAndSelect(page, targets.monster, 'monster');
  await waitReady(page, { selectedId: targets.monster.record_id, npc: false, monster: true });
  await page.locator('#mobile-inspector-toggle').click();
  const inspector = page.locator('#creature-inspector');
  await inspector.waitFor({ state: 'visible', timeout: 30_000 });
  const text = await inspector.innerText();
  assert.match(text, new RegExp(escapeRegex(targets.monster.label), 'i'));
  assert.ok(text.includes(targets.monster.record_id));
  assert.ok(text.includes(expectedDigest));
  assert.match(text, /Monster \/ Spawn/i);
  assert.match(text, /Static marker fallback/i);
  const selectedParams = new URL(page.url()).searchParams;
  assert.equal(selectedParams.get('creature'), targets.monster.record_id);
  assert.equal(selectedParams.get('creatures'), 'monster');
  await page.screenshot({ path: `${evidenceDir}/mobile-monster-only.png`, fullPage: true });
  await context.close();
  return { initialVisibleRecords: initial.visibleRecords, selected: targets.monster.record_id };
}

(async () => {
  const browser = await chromium.launch({
    headless: true,
    args: [
      '--no-sandbox',
      '--disable-dev-shm-usage',
      '--use-angle=swiftshader',
      '--enable-unsafe-swiftshader',
    ],
  });
  try {
    const desktop = await runDesktop(browser);
    process.stdout.write('desktop=PASS\n');
    const mobile = await runMobile(browser);
    process.stdout.write('mobile=PASS\n');
    const result = {
      status: 'PASS',
      atlasRevision: expectedRevision,
      gameSemanticDigest: expectedDigest,
      desktop,
      mobile,
    };
    fs.writeFileSync(`${evidenceDir}/result.json`, `${JSON.stringify(result, null, 2)}\n`);
  } finally {
    await browser.close();
  }
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
