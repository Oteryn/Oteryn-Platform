import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, expect } from '../fixtures.mjs';
import * as OTPAuth from 'otpauth';
import { login, register, submit, waitForWikiBudget } from '../helpers.mjs';
import { loginWithRecoveryCode, query } from '../staff-rbac-helpers.mjs';

const read = name => JSON.parse(fs.readFileSync(new URL(`../${name}.json`, import.meta.url), 'utf8'));
const routes = read('inventory').routes.filter(route => route.methods.includes('GET'));
const expectations = new Map(read('route-expectations').filter(row => row.method === 'GET')
  .map(row => [row.route, row]));
const root = fileURLToPath(new URL('../../', import.meta.url));
const normalize = text => text.replace(/\s+/gu, ' ').trim();

function catalogues(directory, keys, labels) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const filename = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      catalogues(filename, keys, labels);
      continue;
    }
    const locale = /(?:^|[._])(pl|en)\.ftl$/u.exec(entry.name)?.[1];
    if (locale) {
      for (const [, key, value] of fs.readFileSync(filename, 'utf8').matchAll(/^([\w-]+)[ \t]*=([^\n]*)$/gmu)) {
        keys.add(key);
        if (!/[{}]/u.test(value) && value.trim()) labels[locale].add(normalize(value));
      }
    } else if (/\.(html|rs)$/u.test(entry.name)) {
      const source = fs.readFileSync(filename, 'utf8');
      for (const [, first, a, b] of source.matchAll(/\{%\s*if\s+(?:chrome\.)?lang\s*==\s*"(pl|en)"\s*%\}([^{}<>]+)\{%\s*else\s*%\}([^{}<>]+)\{%\s*endif\s*%\}/gu)) {
        labels[first].add(normalize(a));
        labels[first === 'pl' ? 'en' : 'pl'].add(normalize(b));
      }
      for (const [, key] of source.matchAll(/\.(?:text|t|label)\(\s*"([a-z][a-z0-9_.-]+)"\s*\)/gu)) {
        keys.add(key);
      }
    }
  }
}

const keys = new Set();
const labels = { pl: new Set(), en: new Set() };
for (const directory of ['crates', 'templates', 'bin']) catalogues(path.join(root, directory), keys, labels);

// Use the same negative route witnesses as routes.spec for unavailable resources.
// A missing-resource response qualifies only if it actually renders a document.
function targetFor(route, locale) {
  return route.replaceAll('{lang}', locale)
    .replaceAll('{kind}', route.includes('stats/rankings') ? 'yesterday' : route.includes('rankings') ? 'experience' : 'items')
    .replaceAll('{tab}', 'experience')
    .replaceAll('{slug}', route.startsWith('/admin/pages/') ? 'rules' : 'missing-e2e')
    .replace(/\{(?:floor|zoom|x|y)\}/gu, '0')
    .replace(/\{(?:name|slug|key|family|channel|filename|\*target)\}/gu, 'missing-e2e')
    .replace(/\{[^}]+\}/gu, '019a541b-a000-7000-8000-000000000001');
}

async function assertLocale(page, locale, witness) {
  expect.soft(await page.locator('html').getAttribute('lang'), witness).toBe(locale);
  const strings = await page.evaluate(() => {
    const selectors = 'header, nav, footer, a[href="#main"], main h1, main h2, main h3, main label, main button, main option, main legend, main th, main dt, main [role=status]';
    const strings = [document.title];
    for (const node of document.querySelectorAll(selectors)) {
      const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
      for (let text = walker.nextNode(); text; text = walker.nextNode()) {
        if (!text.parentElement.closest('code, time, [translate=no]')
            && text.parentElement.checkVisibility({ visibilityProperty: true })) strings.push(text.data);
      }
    }
    for (const element of document.querySelectorAll('[aria-label], [placeholder], [data-close-label], img[alt]')) {
      if (!element.checkVisibility({ visibilityProperty: true })) continue;
      for (const attribute of ['aria-label', 'placeholder', 'data-close-label', 'alt']) {
        if (element.hasAttribute(attribute)) strings.push(element.getAttribute(attribute));
      }
    }
    return [...new Set(strings)];
  });
  const opposite = locale === 'en' ? 'pl' : 'en';
  for (const value of new Set(strings.map(normalize).filter(Boolean))) {
    // Single-word keys routinely coincide with valid product terms (e.g. Premium).
    const raw = /^[a-z][a-z0-9]*[-_.][a-z0-9_.-]+$/u.test(value) && keys.has(value);
    if (raw) expect.soft(raw, `${witness}: raw key in UI chrome`).toBe(false);
    const foreign = labels[opposite].has(value) && !labels[locale].has(value)
      && value.split(' ').length >= 2;
    if (foreign) expect.soft(foreign, `${witness}: unexpected ${opposite} label ${JSON.stringify(value)}`).toBe(false);
  }
}

let playerContext;
let staffContext;

test.beforeAll(async ({ browser }, info) => {
  test.setTimeout(300_000);
  const locale = info.project.metadata.locale;
  playerContext = await browser.newContext({
    locale,
    viewport: info.project.use.viewport,
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443',
    ignoreHTTPSErrors: true,
  });
  const player = await playerContext.newPage();
  await login(player, locale, await register(player, locale));
  expect((await player.goto(`/${locale}/account/security`)).status()).toBe(200);
  await player.close();
  staffContext = await browser.newContext({
    locale,
    viewport: info.project.use.viewport,
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443',
    ignoreHTTPSErrors: true,
  });
  const staff = await staffContext.newPage();
  const account = await register(staff, locale);
  await query("INSERT INTO account_roles(account_id,role_id) SELECT a.id,r.id FROM accounts a CROSS JOIN roles r WHERE a.email=$1 AND r.key='platform_admin'", [account.email]);
  await login(staff, locale, account);
  expect((await staff.goto(`/admin?lang=${locale}`)).status()).toBe(403);
  await staff.goto(`/${locale}/mfa`);
  await submit(staff, 'form[action$="/mfa/enroll"]', { password: account.password });
  const totp = new OTPAuth.TOTP({
    secret: await staff.locator('main .op-prose').innerText(), algorithm: 'SHA1', digits: 6, period: 30,
  });
  await submit(staff, 'form[action$="/mfa/confirm"]', { password: account.password, code: totp.generate() });
  const code = staff.locator('textarea[readonly]').first();
  await expect(code).toBeVisible();
  await loginWithRecoveryCode(staff, locale, account, await code.inputValue());
  expect((await staff.goto(`/admin?lang=${locale}`)).status()).toBe(200);
  await staff.close();
});

test.afterAll(async () => {
  await staffContext?.close();
  await playerContext?.close();
});

test('locale parity across the GET route inventory', async ({ page: anonymous }, info) => {
  test.setTimeout(1_200_000);
  const locale = info.project.metadata.locale;
  const player = await playerContext.newPage();
  const staff = await staffContext.newPage();
  for (const page of [anonymous, player, staff]) {
    await page.setExtraHTTPHeaders({ 'accept-language': locale });
  }
  const checked = new Map();
  try {
    for (const [actor, page] of [['anonymous', anonymous], ['player', player], ['staff', staff]]) {
      let documents = 0;
      for (const { route } of routes) {
        const expected = expectations.get(route);
        expect(expected, `GET inventory route requires a harness expectation: ${route}`).toBeDefined();
        const target = new URL(targetFor(route, locale), process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443');
        const witness = `${actor} ${locale} ${route}`;
        await test.step(witness, async () => {
          await waitForWikiBudget(target.href, 2);
          const options = { headers: { accept: 'text/html', 'accept-language': locale } };
          const entry = await page.request.get(target.href, { ...options, maxRedirects: 0 });
          let probe = entry;
          try {
            const status = entry.status();
            const redirect = status >= 300 && status < 400 && entry.headers().location;
            if (actor !== expected.actor) {
              const path = redirect && new URL(redirect, target.href).pathname;
              const loginGate = path && /^\/(?:pl\/|en\/)?(?:login|mfa\/(?:login|challenge))$/u.test(path);
              // The route itself is inaccessible to this actor. Its login
              // destination is independently inventoried for anonymous access.
              if (status === 401 || status === 403 || loginGate) return;
            }
            if (redirect) probe = await page.request.get(target.href, options);
            if (actor === expected.actor) expect.soft(probe.status(), witness).toBe(expected.status);
            expect.soft(probe.status(), witness).not.toBe(429);
            expect.soft(probe.status(), witness).toBeLessThan(504);
            if (!probe.headers()['content-type']?.startsWith('text/html')) return;
            // Widget fragments can be HTML without a document. Every full
            // document has a locale contract, including cold refusals.
            if (!/<html[\s>]/iu.test(await probe.text())) return;
            const response = await page.goto(target.href, { waitUntil: 'domcontentloaded' });
            expect.soft(response.status(), witness).toBe(probe.status());
            expect.soft(response.headers()['content-type'], witness).toMatch(/^text\/html/u);
            await assertLocale(page, locale, witness);
            documents++;
          } finally {
            if (probe !== entry) await probe.dispose();
            await entry.dispose();
          }
        });
      }
      checked.set(actor, documents);
      expect.soft(documents, `${actor} must reach HTML documents`).toBeGreaterThan(0);
    }
  } finally {
    await player.close();
    await staff.close();
    await info.attach('locale-documents', {
      body: JSON.stringify(Object.fromEntries(checked)), contentType: 'application/json',
    });
  }
});
