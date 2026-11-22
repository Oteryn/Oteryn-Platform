import crypto from 'node:crypto';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {test, expect} from '../fixtures.mjs';
import {csrfRejected, login, password, submit, waitForLoginBudget} from '../helpers.mjs';
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
  const suffix = crypto.randomBytes(8).toString('hex').match(/.{4}/g).join(':');
  return {'x-forwarded-for': `2001:db8::${suffix}`};
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

async function startQuotaWindow(seconds) {
  await expect.poll(async () => (await query(
    'SELECT mod(floor(extract(epoch FROM clock_timestamp()))::bigint, $1)::int AS elapsed',
    [seconds],
  ))[0].elapsed, {timeout: 35_000, intervals: [1000]}).toBeLessThan(seconds - 25);
}

test('personal export requires sudo and throttles repeat downloads', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const target = `/${lang}/account/privacy/export`;
  const anonymous = await page.request.get(target, {maxRedirects: 0});
  expect(anonymous.status()).toBe(303);
  expect(anonymous.headers().location).toBe(`/${lang}/login`);
  const malformed = await page.request.head(target, {
    headers: {cookie: '__Host-oteryn_session=malformed'},
    maxRedirects: 0,
  });
  expect(malformed.status()).toBe(401);

  const account = await loginSeededAccount(page, lang);
  expect((await page.request.get(target)).status()).toBe(403);
  const source = testSource();
  await enableSudo(page, lang, account);
  await startQuotaWindow(900);
  for (let index = 0; index < 3; index++) {
    const response = await page.request.get(target, {headers: source});
    expect(response.status()).toBe(200);
    expect(response.headers()['content-disposition']).toBe('attachment; filename="account-data.json"');
    expect(response.headers()['cache-control']).toContain('no-store');
    expect((await response.json()).account.id).toBe(account.id);
  }
  const refused = await page.request.get(target, {headers: testSource()});
  await expectThrottle(refused, lang, 900);
  expect(refused.headers()['content-disposition']).toBeUndefined();
});

test('profile comment validates submissions and refuses writes over quota', async ({page}, info) => {
  test.setTimeout(110_000);
  const lang = info.project.metadata.locale;
  const account = await loginSeededAccount(page, lang);
  const {target, form, options} = await profileForm(page, lang, account);
  await csrfRejected(page, target, {comment: 'Missing CSRF'});
  const source = testSource();
  await startQuotaWindow(60);
  expect((await page.request.post(target, options('a'.repeat(501), source))).status()).toBe(422);
  for (let index = 1; index <= 9; index++) {
    const response = await page.request.post(target, options(`Allowed comment ${index}`, source));
    expect(response.status()).toBe(303);
    expect(response.headers().location).toBe(target);
  }
  await expectThrottle(await page.request.put(target, options('Refused comment', testSource())), lang, 60);
  expect((await page.reload()).status()).toBe(200);
  await expect(form.locator('[name=comment]')).toHaveValue('Allowed comment 9');
});

test('the same IP exhausts each budget across distinct authenticated accounts', async ({browser}, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const exportTarget = `/${lang}/account/privacy/export`;
  const sharedSource = testSource();
  const contexts = [];

  try {
    const actors = [];
    for (let index = 0; index < 5; index++) {
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
      expect(await postAuthForm(page, `/${lang}/account/sudo`, {
        current_password: account.password,
      })).toBe(`/${lang}/account/security`);
      const profile = index < 4 ? await profileForm(page, lang, account) : null;
      actors.push({page, account, profile});
    }

    await startQuotaWindow(900);
    for (const {page, account} of actors.slice(0, 4)) {
      for (let index = 0; index < 3; index++) {
        const response = await page.request.get(exportTarget, {headers: sharedSource});
        expect(response.status()).toBe(200);
        expect((await response.json()).account.id).toBe(account.id);
      }
    }
    const exporter = actors[4];
    await expectThrottle(await exporter.page.request.get(exportTarget, {headers: sharedSource}), lang, 900);
    const exportFromAnotherSource = await exporter.page.request.get(exportTarget, {headers: testSource()});
    expect(exportFromAnotherSource.status()).toBe(200);
    expect((await exportFromAnotherSource.json()).account.id).toBe(exporter.account.id);

    await startQuotaWindow(60);
    for (const {page, profile: {target, options}} of actors.slice(0, 3)) {
      for (let index = 0; index < 10; index++) {
        expect((await page.request.post(target, options(`Comment ${index}`, sharedSource))).status()).toBe(303);
      }
    }
    const {page, profile: {target, options, form}} = actors[3];
    await expectThrottle(await page.request.post(target, options('Refused by IP', sharedSource)), lang, 60);
    expect((await page.reload()).status()).toBe(200);
    await expect(form.locator('[name=comment]')).toHaveValue('');
    expect((await page.request.post(target, options('Allowed from another IP', testSource()))).status()).toBe(303);
    expect((await page.reload()).status()).toBe(200);
    await expect(form.locator('[name=comment]')).toHaveValue('Allowed from another IP');
  } finally {
    await Promise.all(contexts.map(context => context.close()));
  }
});
