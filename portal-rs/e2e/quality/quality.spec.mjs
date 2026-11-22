import AxeBuilder from '@axe-core/playwright';
import axeCore from 'axe-core';
import { test, expect, request as playwrightRequest } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import fs from 'node:fs';
import { login, password, seedPasswordRange, submit, waitForWikiBudget } from '../helpers.mjs';
import { loginWithRecoveryCode, query } from '../staff-rbac-helpers.mjs';
import { documentLanguage, languageFilter, matchingRoute, maxSamples, routes } from './routes.mjs';
import { assertKeyboard } from './keyboard.mjs';
import { publishEditorialFixtures } from './editorial-fixtures.mjs';
import { assertBudgets, collectResponses, installVitals, startPaintTrace, vitals } from './performance.mjs';

const actors = ['platform_admin', 'anonymous', 'player'];
const selected = process.env.PORTAL_QUALITY_ROUTE?.split(',').map(pattern => pattern.trim()) ?? [];
const editorialRoutes = ['/{lang}/news/{slug}', '/{lang}/events/{slug}', '/{lang}/wiki/articles/{slug}'];
const wcag = ['wcag2a', 'wcag2aa'];
const requiredRules = ['color-contrast', 'label', 'bypass', 'select-name',
  'landmark-one-main', 'landmark-unique', 'region'];
const wcagRules = new Set(axeCore.getRules(wcag).map(rule => rule.ruleId));
const auditRules = [...new Set([...wcagRules, ...requiredRules])];
const format = violations => violations.map(({ id, impact, nodes }) => ({
  id, impact, targets: nodes.map(({ target }) => target),
}));

function seed(entity, input) {
  const binary = process.env.PORTAL_QUALITY_SEED_BINARY;
  const file = process.env.PORTAL_QUALITY_SEED_ENV;
  if (!binary || !file) throw new Error('Use node quality/run.mjs to start the real harness');
  const env = JSON.parse(fs.readFileSync(file));
  try {
    return JSON.parse(execFileSync(binary, ['seed', entity, JSON.stringify(input)], {
      env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'],
    }));
  } catch {
    throw new Error(`Portal seed ${entity} failed`);
  }
}

async function actorState(browser, info, actor) {
  const fixturePassword = password();
  await seedPasswordRange(fixturePassword);
  if (actor === 'anonymous') return { cookies: [], origins: [] };
  const locale = info.project.metadata.locale;
  const credentials = { email: `quality-${randomUUID()}@example.invalid`, password: fixturePassword };
  const { accountId } = seed('account', { ...credentials, locale });
  const context = await browser.newContext({
    ...info.project.use, baseURL: process.env.PORTAL_E2E_ORIGIN,
  });
  let phase = 'sign-in';
  let editorial = [];
  try {
    const page = await context.newPage();
    if (actor === 'platform_admin') {
      const roles = await query(
        "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin' RETURNING role_id",
        [accountId],
      );
      expect(roles).toHaveLength(1);
      const { recoveryCodes } = seed('account-mfa', { accountId, password: credentials.password });
      await loginWithRecoveryCode(page, locale, credentials, recoveryCodes[0]);
      expect((await page.goto('/admin')).status(), 'platform admin reaches staff panel').toBe(200);
      expect(new URL(page.url()).pathname).toMatch(/^\/admin(?:\/|$)/);
    } else {
      await login(page, locale, credentials);
      expect((await page.goto('/account')).status(), 'seeded player reaches account').toBe(200);
      expect(new URL(page.url()).pathname).toMatch(/\/account\/?$/);
    }
    if (actor === 'platform_admin' &&
        (!selected.length || editorialRoutes.some(pattern => selected.includes(pattern)))) {
      phase = 'editorial publication';
      editorial = await publishEditorialFixtures(page, locale);
    }
    phase = 'ticket creation';
    expect((await page.goto(`/${locale}/support/tickets/create`)).status()).toBe(200);
    await submit(page, 'main form', {
      subject: 'Quality route coverage', body: 'Synthetic ticket for the quality browser suite.',
    });
    await expect(page).toHaveURL(/support\/tickets\/[a-f0-9-]+$/);
    const ticket = new URL(page.url()).pathname.split('/').at(-1);
    phase = 'report creation';
    expect((await page.goto(`/${locale}/support/reports/create`)).status()).toBe(200);
    await submit(page, 'main form', {
      target_reference: accountId, evidence_summary: 'Synthetic report for the quality browser suite.',
    });
    await expect(page).toHaveURL(/support\/reports\/[a-f0-9-]+$/);
    const report = new URL(page.url()).pathname.split('/').at(-1);
    return { storage: await context.storageState(), accountId, ticket, report, editorial };
  } catch {
    throw new Error(`Could not prepare the ${actor} browser fixture during ${phase}`);
  } finally {
    await context.close();
  }
}

function addLinks(inventory, links, source) {
  for (const link of links) {
    const url = new URL(link);
    if (url.origin !== source.origin || url.pathname.startsWith('/language/')) continue;
    if (url.pathname === source.pathname && url.search === source.search) continue;
    const row = matchingRoute(inventory, url.pathname);
    if (row) {
      const target = url.pathname + url.search;
      if (!row.paths.includes(target) && row.paths.length < maxSamples) row.paths.push(target);
    } else {
      // Constant-based routes and composed routers can be absent from the source scan.
      const pattern = url.pathname.replace(/^\/(?:pl|en)(?=\/|$)/, '/{lang}')
        .replace(/(?<=\/)[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}(?=\/|$)/gi, '{id}');
      inventory.push({ pattern, paths: [url.pathname + url.search], documentedStatuses: [] });
    }
  }
}

async function discover(probe, entry, target, covered) {
  const origin = process.env.PORTAL_E2E_ORIGIN;
  await waitForWikiBudget(target, 2);
  let requested = target;
  let response;
  for (let hop = 0; hop < 10; hop++) {
    response = await probe.get(requested, { maxRedirects: 0 });
    const location = response.headers().location;
    if (![301, 302, 303, 307, 308].includes(response.status()) || !location) break;
    const url = new URL(location, response.url());
    const status = response.status();
    await response.dispose();
    response = undefined;
    if (url.origin !== origin) {
      return { record: {
        pattern: entry.pattern, requestedPath: new URL(target, origin).pathname,
        status, outcome: 'external redirect',
      } };
    }
    requested = url.href;
  }
  expect(response, 'local redirect chain terminates').toBeDefined();
  try {
    const url = new URL(response.url());
    expect(url.origin, 'local document origin').toBe(origin);
    const status = response.status();
    const label = url.pathname;
    const record = { pattern: entry.pattern, requestedPath: new URL(target, origin).pathname, path: label, status };
    expect.soft(status === 503 || status < 500, `${label}: navigation response`).toBe(true);
    const headers = response.headers();
    if (!headers['content-type']?.startsWith('text/html')) return { record: { ...record, outcome: 'non-document' } };
    if (status === 503) {
      if (entry.documentedStatuses.length) {
        expect.soft(entry.documentedStatuses, `${label}: documented cold response`).toContain(503);
      }
      expect.soft(headers['cache-control'], `${label}: 503 cache policy`).toContain('no-store');
    }
    const markup = await response.text();
    if (!/<html[\s>]/i.test(markup)) return { record: { ...record, outcome: 'HTML fragment' } };
    // The shared refusal renderer can return the same document for many
    // resources. Keep each route outcome and audit each distinct refusal.
    const key = status >= 400
      ? JSON.stringify([status, headers['content-language'], createHash('sha256').update(markup).digest('hex')])
      : status + ' ' + label + url.search;
    if (covered.has(key)) return { record: { ...record, outcome: 'already audited', auditedAt: covered.get(key) } };
    return { record, key };
  } finally {
    await response.dispose();
  }
}

async function assessPage(browser, info, state, entry, target, covered, inventory, probeOptions) {
  const origin = process.env.PORTAL_E2E_ORIGIN;
  const probe = await playwrightRequest.newContext(probeOptions);
  let discovery;
  try {
    discovery = await discover(probe, entry, target, covered);
  } finally {
    await probe.dispose();
  }
  const { record, key } = discovery;
  if (record.outcome) return record;
  const { status, path: label } = record;
  const context = await browser.newContext({ ...info.project.use, baseURL: origin, storageState: state });
  let stopTrace;
  try {
    const page = await context.newPage();
    await installVitals(page);
    stopTrace = await startPaintTrace(page);
    const transfers = collectResponses(page);
    let response;
    let attachment;
    page.on('download', download => { attachment = download; });
    try {
      response = await page.goto(target);
    } catch (error) {
      if (!error.message.includes('net::ERR_ABORTED')) throw error;
      attachment ??= await page.waitForEvent('download', { timeout: 3000 });
      await attachment.cancel();
      return { pattern: entry.pattern, requestedPath: new URL(target, origin).pathname, outcome: 'download' };
    }
    expect(response, 'navigation produced a response').not.toBeNull();
    const url = new URL(page.url());
    expect(url.origin, 'local document origin').toBe(origin);
    expect(response.status(), `${label}: browser and source discovery agree on status`).toBe(status);
    expect(url.pathname, 'browser and source discovery agree on destination').toBe(label);
    await vitals(page);
    expect.soft(await page.locator('html').getAttribute('lang'), `${label}: document language`)
      .toBe(documentLanguage(url, entry.pattern, info.project.metadata.locale, status));
    await expect.soft(page.getByRole('main'), `${label}: main landmark`).toHaveCount(1);

    const scanner = new AxeBuilder({ page }).withRules(auditRules);
    // A single document can use axe.run directly. Keep the frame-aware
    // aggregation path for pages that embed other documents.
    if (page.frames().length === 1) scanner.setLegacyMode();
    const axe = await scanner.analyze();
    const serious = format(axe.violations.filter(({ id, impact }) =>
      wcagRules.has(id) && ['serious', 'critical'].includes(impact)));
    const details = format(axe.violations.filter(({ id }) => requiredRules.includes(id)));
    expect.soft(serious, `${label}: WCAG 2 A/AA serious or critical`).toEqual([]);
    expect.soft(details, `${label}: contrast, labels, landmarks and bypass`).toEqual([]);
    const links = await page.evaluate(() => [
      ...[...document.querySelectorAll('a[href]:not([hreflang],[download])')].map(anchor => anchor.href),
      ...[...document.forms].filter(form => form.method === 'get').map(form => form.action),
    ]);
    addLinks(inventory, links, url);
    const measurement = await page.evaluate(() => window.portalQualityVitals());
    const paintTrace = await stopTrace?.();
    stopTrace = undefined;
    const selectedLanguage = languageFilter(url, status);
    if (selectedLanguage) {
      const labelText = selectedLanguage === 'en' ? 'Skip to content' : 'Przejdź do treści';
      await expect.soft(page.locator('a[href="#main"]').first(), `${label}: skip link language`).toHaveText(labelText);
    }
    await assertKeyboard(page, label);
    const resources = await transfers();
    const totals = assertBudgets(measurement, resources, label);
    covered.set(key, label);
    return { ...record, outcome: 'audited', ...measurement, paintTrace, ...totals,
      largest: Math.max(0, ...resources.map(resource => resource.bytes)), serious, details };
  } finally {
    try {
      await stopTrace?.();
    } finally {
      await context.close();
    }
  }
}

for (const actor of actors) {
  test(`quality budgets for ${actor}`, async ({ browser }, info) => {
    const state = await actorState(browser, info, actor);
    const locale = info.project.metadata.locale;
    const inventory = routes(locale, {
      player: state.accountId, ticket: state.ticket, report: state.report, world: process.env.PORTAL_QUALITY_WORLD,
    });
    const origin = new URL(process.env.PORTAL_E2E_ORIGIN);
    addLinks(inventory, (state.editorial ?? []).map(path => new URL(path, origin)), origin);
    const covered = new Map();
    const evidence = [];
    const report = info.outputPath('quality-inventory.json');
    for (const pattern of selected) {
      expect(inventory.some(row => row.pattern === pattern), `selected source pattern ${pattern} exists`).toBe(true);
    }
    const probeOptions = {
      baseURL: process.env.PORTAL_E2E_ORIGIN,
      ignoreHTTPSErrors: true,
      storageState: state.storage ?? state,
      extraHTTPHeaders: { accept: 'text/html', 'accept-language': locale },
    };
    try {
      const tested = new Map();
      let addedSamples;
      do {
        addedSamples = false;
        for (const entry of inventory) {
          if (selected.length && !selected.includes(entry.pattern)) continue;
          for (let index = tested.get(entry) ?? 0; index < entry.paths.length; index++) {
            const target = entry.paths[index];
            tested.set(entry, index + 1);
            addedSamples = true;
            try {
              evidence.push(await assessPage(browser, info, state.storage ?? state, entry, target, covered, inventory, probeOptions));
            } catch (error) {
              // Browser exceptions can include full URLs. Preserve the category and route without query data.
              const requestedPath = new URL(target, origin).pathname;
              const reason = error.message.match(/net::[A-Z_0-9]+/)?.[0] ?? error.name;
              evidence.push({ pattern: entry.pattern, requestedPath, outcome: 'audit error', error: reason });
              expect.soft(reason, `${requestedPath}: audit completed`).toBe('');
            }
            fs.writeFileSync(report, JSON.stringify({ actor, locale, evidence }), { mode: 0o600 });
          }
        }
      } while (addedSamples);
      expect(covered.size, 'rendered documents audited').toBeGreaterThan(0);
    } finally {
      await info.attach('quality-inventory', {
        contentType: 'application/json', body: JSON.stringify({ actor, locale, evidence }),
      });
    }
  });
}
