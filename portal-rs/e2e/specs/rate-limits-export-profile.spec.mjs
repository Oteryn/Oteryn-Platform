import crypto from 'node:crypto';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {test, expect} from '../fixtures.mjs';
import {exportOptions, login, password, submit, waitForLoginBudget} from '../helpers.mjs';
import {query} from '../staff-rbac-helpers.mjs';

function seedAccount(locale) {
  const account = {email: `limits-${crypto.randomUUID()}@example.invalid`, password: password(), locale};
  const binary = process.env.PORTAL_E2E_BINARY
    || path.resolve(process.env.CARGO_TARGET_DIR || '../target', 'debug/oteryn-portal');
  const result = spawnSync(binary, ['seed', 'account', JSON.stringify(account)], {encoding: 'utf8'});
  expect(result.status, 'disposable account seed succeeded').toBe(0);
  return {...account, id: JSON.parse(result.stdout).accountId};
}

async function expectThrottle(response, lang, window) {
  expect(response.status()).toBe(429);
  const headers = response.headers();
  expect(headers['content-language']).toBe(lang);
  expect(headers['content-type']).toContain('text/html');
  expect(headers['cache-control']).toContain('no-store');
  expect(headers['retry-after']).toMatch(/^[1-9]\d*$/);
  expect(Number(headers['retry-after'])).toBeLessThanOrEqual(window);
  expect(await response.text()).toContain(lang === 'pl' ? 'Zbyt wiele żądań' : 'Too many requests');
}

function testSource() {
  const suffix = crypto.randomBytes(8).toString('hex').match(/.{4}/g).map(group => (parseInt(group, 16) || 1).toString(16)).join(':');
  return {'x-forwarded-for': `2001:db8::${suffix}`};
}

function quotaHash(policy, key) {
  const hash = crypto.createHash('sha256');
  for (const part of [policy, key]) {
    const length = Buffer.alloc(8);
    length.writeBigUInt64BE(BigInt(Buffer.byteLength(part)));
    hash.update(length).update(part);
  }
  return hash.digest();
}

async function quotaCounters(policy, account, source) {
  return query(`
    SELECT limiter_name, window_start::text, count::text
      FROM web_rate_limits
     WHERE (limiter_name = $1 AND key_hash = $2)
        OR (limiter_name = $3 AND key_hash = $4)
     ORDER BY limiter_name, window_start
  `, [policy, quotaHash(policy, account.id), `${policy}-source`,
    quotaHash(`${policy}-source`, source['x-forwarded-for'])]);
}

async function loginSeededAccount(page, lang) {
  const account = seedAccount(lang);
  await login(page, lang, account);
  return account;
}

async function enableSudo(page, lang, account) {
  await page.goto(`/${lang}/account/sudo`);
  await submit(page, 'main form', {current_password: account.password});
}

async function postAuthForm(page, target, values) {
  const shown = await page.goto(target, {waitUntil: 'domcontentloaded'});
  expect(shown.status()).toBe(200);
  const form = page.locator('main form');
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const action = await form.getAttribute('action') || target;
  const response = await page.request.post(action, {
    form: {...fields, ...values},
    headers: {origin: new URL(page.url()).origin},
    maxRedirects: 0,
  });
  expect(response.status()).toBe(302);
  return response.headers().location;
}

async function profileForm(page, lang, account) {
  const [world] = await query("SELECT id FROM world_status WHERE name = 'Local Bazaar World'");
  const character = (await query('SELECT uuidv7() AS id'))[0].id;
  const publication = await fetch(`${process.env.PORTAL_E2E_GAME_CLOSURE_CONTROL_URL}/portfolio`, {
    method: 'POST',
    headers: {'content-type': 'application/json'},
    body: JSON.stringify({account_id: account.id, character_id: character, world_id: world.id}),
  });
  expect(publication.status).toBe(200);
  const {name} = await publication.json();
  const target = `/${lang}/account/characters/${encodeURIComponent(name)}/profile`;
  expect((await page.goto(target, {waitUntil: 'domcontentloaded'})).status()).toBe(200);
  const form = page.locator('main form');
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const options = (comment, source) => ({
    form: {...fields, comment},
    headers: {origin: new URL(page.url()).origin, ...source},
    maxRedirects: 0,
  });
  return {target, form, options};
}

const QUOTA_ROLLOVER_TIMEOUT = 35_000;

async function startQuotaWindow(seconds) {
  await expect.poll(async () => (await query(
    'SELECT mod(floor(extract(epoch FROM clock_timestamp()))::bigint, $1)::int AS elapsed',
    [seconds],
  ))[0].elapsed, {timeout: QUOTA_ROLLOVER_TIMEOUT, intervals: [1000]}).toBeLessThan(seconds - 25);
}

test('personal export requires sudo and throttles repeat downloads', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const target = `/${lang}/account/privacy/export`;
  const anonymous = await page.request.get(target, {maxRedirects: 0});
  expect(anonymous.status()).toBe(303);
  expect(anonymous.headers().location).toBe(`/${lang}/login`);
  // The browser account gate treats a malformed session cookie as anonymous.
  const invalidSession = await page.request.head(target, {
    headers: {cookie: '__Host-oteryn_session=malformed'},
    maxRedirects: 0,
  });
  expect(invalidSession.status()).toBe(303);
  expect(invalidSession.headers().location).toBe(`/${lang}/login`);
  expect(invalidSession.headers()['content-disposition']).toBeUndefined();

  const account = await loginSeededAccount(page, lang);
  const passwordChallenge = await page.request.get(target, {maxRedirects: 0});
  expect(passwordChallenge.status()).toBe(303);
  expect(passwordChallenge.headers().location).toBe(`/${lang}/account/sudo?return_to=export`);
  expect(passwordChallenge.headers()['content-disposition']).toBeUndefined();
  const source = testSource();
  const counters = () => quotaCounters('identity-data-export', account, source);
  await page.goto(`/${lang}/account/sudo`);
  const csrf = await page.locator('main form [name=_csrf]').inputValue();
  const origin = new URL(page.url()).origin;
  const beforeSudo = await counters();
  expect((await page.request.post(target, {
    form: {_csrf: csrf}, headers: {origin, ...source}, maxRedirects: 0,
  })).status()).toBe(403);
  expect(await counters()).toEqual(beforeSudo);

  await enableSudo(page, lang, account);
  const post = await exportOptions(page, lang);
  const beforeProbes = await counters();
  for (const method of ['GET', 'HEAD']) {
    const response = await page.request.fetch(target, {
      method, headers: {...source, 'sec-fetch-site': 'cross-site'}, maxRedirects: 0,
    });
    expect(response.status()).toBe(200);
    expect(response.headers()['content-disposition']).toBeUndefined();
    expect(await counters()).toEqual(beforeProbes);
  }
  expect((await page.request.post(target, {
    ...post, form: {}, headers: {...post.headers, ...source},
  })).status()).toBe(403);
  expect(await counters()).toEqual(beforeProbes);
  expect((await page.request.post(target, {
    ...post, headers: {...post.headers, ...source, 'sec-fetch-site': 'cross-site'},
  })).status()).toBe(403);
  expect(await counters()).toEqual(beforeProbes);
  await startQuotaWindow(900);
  for (let index = 0; index < 3; index++) {
    const response = await page.request.post(target, {...post, headers: {...post.headers, ...source}});
    expect(response.status()).toBe(200);
    expect(response.headers()['content-disposition']).toBe('attachment; filename="account-data.json"');
    expect(response.headers()['cache-control']).toContain('no-store');
    expect((await response.json()).account.id).toBe(account.id);
  }
  expect((await counters()).map(row => row.count)).toEqual(['3', '3']);
  const refused = await page.request.post(target, {...post, headers: {...post.headers, ...testSource()}});
  await expectThrottle(refused, lang, 900);
  expect(refused.headers()['content-disposition']).toBeUndefined();
});

test('profile comment validates submissions and refuses writes over quota', async ({page}, info) => {
  test.setTimeout(110_000);
  const lang = info.project.metadata.locale;
  const account = await loginSeededAccount(page, lang);
  const {target, form, options} = await profileForm(page, lang, account);
  const source = testSource();
  const counters = () => quotaCounters('character-profile-update', account, source);
  const before = await counters();
  const missing = options('Missing CSRF', source);
  delete missing.form._csrf;
  expect((await page.request.post(target, missing)).status()).toBe(403);
  expect(await counters()).toEqual(before);
  await startQuotaWindow(60);
  expect((await page.request.post(target, options('a'.repeat(501), source))).status()).toBe(422);
  for (let index = 1; index <= 9; index++) {
    const response = await page.request.post(target, options(`Allowed comment ${index}`, source));
    expect(response.status()).toBe(303);
    expect(response.headers().location).toBe(target);
  }
  expect((await counters()).map(row => row.count)).toEqual(['10', '10']);
  await expectThrottle(await page.request.put(target, options('Refused comment', testSource())), lang, 60);
  expect((await page.reload()).status()).toBe(200);
  await expect(form.locator('[name=comment]')).toHaveValue('Allowed comment 9');
});

const sharedIpTest = test.extend({
  openActor: async ({browser}, use, info) => {
    const contexts = [];
    const lang = info.project.metadata.locale;
    try {
      await use(async () => {
        const context = await browser.newContext({
          baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true,
          locale: lang, viewport: info.project.use.viewport,
        });
        contexts.push(context);
        const page = await context.newPage();
        const account = seedAccount(lang);
        await waitForLoginBudget();
        expect(await postAuthForm(page, `/${lang}/login`, {
          email: account.email, password: account.password,
        })).toBe(`/${lang}/`);
        return {page, account};
      });
    } finally {
      await Promise.all(contexts.map(context => context.close()));
    }
  },
});

sharedIpTest('the same IP exhausts the export budget across distinct authenticated accounts', async ({openActor}, info) => {
  test.setTimeout(240_000 + QUOTA_ROLLOVER_TIMEOUT);
  const lang = info.project.metadata.locale;
  const exportTarget = `/${lang}/account/privacy/export`;
  const sharedSource = testSource();
  const actors = [];
  for (let index = 0; index < 5; index++) {
    const {page, account} = await openActor();
    expect(await postAuthForm(page, `/${lang}/account/sudo`, {
      current_password: account.password,
    })).toBe(`/${lang}/account/security`);
    actors.push({page, account, exportPost: await exportOptions(page, lang)});
  }

  const attemptExport = ({page, exportPost}, source) => page.request.post(exportTarget, {
    ...exportPost, headers: {...exportPost.headers, ...source},
  });
  await startQuotaWindow(900);
  for (const actor of actors.slice(0, 4)) {
    for (let index = 0; index < 3; index++) {
      const response = await attemptExport(actor, sharedSource);
      expect(response.status()).toBe(200);
      expect((await response.json()).account.id).toBe(actor.account.id);
    }
  }
  const exporter = actors[4];
  const freeExportSource = testSource();
  for (let index = 0; index < 2; index++) {
    expect((await attemptExport(exporter, freeExportSource)).status()).toBe(200);
  }
  await expectThrottle(await attemptExport(exporter, sharedSource), lang, 900);
  const exportFromAnotherSource = await attemptExport(exporter, freeExportSource);
  expect(exportFromAnotherSource.status()).toBe(200);
  expect((await exportFromAnotherSource.json()).account.id).toBe(exporter.account.id);
});

sharedIpTest('the same IP exhausts the profile budget across distinct authenticated accounts', async ({openActor}, info) => {
  test.setTimeout(240_000 + QUOTA_ROLLOVER_TIMEOUT);
  const lang = info.project.metadata.locale;
  const sharedSource = testSource();
  const actors = [];
  for (let index = 0; index < 4; index++) {
    const {page, account} = await openActor();
    actors.push({page, account, profile: await profileForm(page, lang, account)});
  }

  const foreignActor = actors[3];
  const foreignCounters = () => quotaCounters('character-profile-update', foreignActor.account, sharedSource);
  const beforeForeign = await foreignCounters();
  const otherTarget = actors[0].profile.target;
  expect((await foreignActor.page.request.post(otherTarget,
    foreignActor.profile.options('Foreign edit', sharedSource))).status()).toBe(404);
  expect(await foreignCounters()).toEqual(beforeForeign);

  await startQuotaWindow(60);
  for (const {page, profile: {target, options}} of actors.slice(0, 3)) {
    for (let index = 0; index < 10; index++) {
      expect((await page.request.post(target, options(`Comment ${index}`, sharedSource))).status()).toBe(303);
    }
  }
  const {page, profile: {target, options, form}} = actors[3];
  await expectThrottle(await page.request.post(target, options('Refused by IP', sharedSource)), lang, 60);
  expect((await page.goto(target)).status()).toBe(200);
  await expect(form.locator('[name=comment]')).toHaveValue('');
  expect((await page.request.post(target, options('Allowed from another IP', testSource()))).status()).toBe(303);
  expect((await page.reload()).status()).toBe(200);
  await expect(form.locator('[name=comment]')).toHaveValue('Allowed from another IP');
});
