import fs from 'node:fs';
import * as OTPAuth from 'otpauth';
import { test, expect } from '../fixtures.mjs';
import { login, register, submit, waitForWikiBudget } from '../helpers.mjs';
import { loginWithRecoveryCode, query } from '../staff-rbac-helpers.mjs';

const read = (path) => fs.readFileSync(new URL(path, import.meta.url), 'utf8');
const inventory = JSON.parse(read('../inventory.json'));
const expectations = JSON.parse(read('../route-expectations.json'));
const workspace = new URL('../../', import.meta.url);
const normalize = (value) => value.normalize('NFC').replace(/\s+/gu, ' ').trim();
const catalogs = Object.fromEntries(['pl', 'en'].map((locale) => [locale, new Map()]));
const keys = new Set();

function readCatalogs(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const url = new URL(entry.name + (entry.isDirectory() ? '/' : ''), directory);
    if (entry.isDirectory()) {
      readCatalogs(url);
      continue;
    }
    const locale = /(?:^|[._])(pl|en)\.ftl$/u.exec(entry.name)?.[1];
    if (!locale) continue;
    for (const [, key, text] of fs.readFileSync(url, 'utf8').matchAll(/^([a-z][\w-]*)\s*=[ \t]*([^\r\n]*)/gmu)) {
      keys.add(key);
      const value = normalize(text);
      if (value && !/[{}]/u.test(value)) {
        const names = catalogs[locale].get(value) ?? [];
        names.push(key);
        catalogs[locale].set(value, names);
      }
    }
  }
}
for (const path of ['crates/', 'templates/']) readCatalogs(new URL(path, workspace));

// Exact labels and longer diagnostic phrases are deliberately chosen to avoid
// flagging words shared by both languages, proper names or editorial content.
const markerKeys = new Set([
  'skip', 'shell-register', 'shell-login', 'shell-logout',
  'login-error', 'reset-error', 'mfa-error', 'error-403-description',
  'error-404-description', 'error-422-description', 'error-503-description',
  'announcements-heading', 'announcements-save', 'coin-checkout-title', 'refused', 'error', 'invalid',
]);
const opposite = (locale) => locale === 'en' ? 'pl' : 'en';
const markers = Object.fromEntries(['pl', 'en'].map((locale) => {
  const unique = [...catalogs[opposite(locale)]]
    .filter(([value]) => !catalogs[locale].has(value))
    .flatMap(([value, names]) => names.map((name) => [value, name]));
  return [locale, {
    body: unique.filter(([, key]) => markerKeys.has(key)),
    title: new Map(unique.filter(([, key]) => /(?:^|-)title(?:-|$)/u.test(key) || ['announcements-heading', 'home-link'].includes(key))),
  }];
}));

function wrongLocale(value, locale, title = false) {
  const text = normalize(value);
  if (title) {
    for (const part of text.split(/\s+[·|–—]\s+/u)) {
      if (markers[locale].title.has(part)) return markers[locale].title.get(part);
    }
  }
  return markers[locale].body.find(([marker]) =>
    text === marker || (/[.!?]$/u.test(marker) && text.includes(marker)))?.[1];
}

function catalogText(path, key) {
  const source = fs.readFileSync(new URL(path, workspace), 'utf8');
  const value = source.split(/\r?\n/u).find((line) => line.startsWith(`${key} = `))?.slice(key.length + 3);
  if (!value) throw new Error(`Missing catalog marker ${key} in ${path}`);
  return value;
}

function assertText(value, locale, scope, title = false) {
  const text = normalize(value);
  const tokens = text.match(/(?<![\w/-])[a-z][\w]*(?:-[\w]+)+(?![\w/-])/gu) ?? [];
  expect.soft(tokens.filter((token) => keys.has(token)), `${scope}: rendered Fluent keys`).toEqual([]);
  expect.soft(/\{\{\s*[\w.]|\{\s*\$[\w-]+/u.test(text), `${scope}: unresolved interpolation`).toBe(false);
  expect.soft(wrongLocale(text, locale, title), `${scope}: opposite-locale catalog marker`).toBeUndefined();
}

async function assertDocument(page, locale, target) {
  expect.soft(await page.locator('html').getAttribute('lang'), `${target}: document language`).toBe(locale);
  const title = await page.title();
  const titleLabel = normalize(title).replace(/\s*[·|–—]\s*Oteryn$/u, '').trim();
  expect.soft(titleLabel.length, `${target}: localized document title`).toBeGreaterThan(0);
  assertText(title, locale, `${target}: title`, true);
  // Text nodes preserve boundaries between labels and include closed menus.
  const values = await page.locator('body').evaluate((body, locale) => {
    const values = [];
    const append = (element, value) => {
      const declaredLanguage = element.closest('[lang]:not(html), [hreflang]');
      const declaredLocale = declaredLanguage?.getAttribute('lang') ?? declaredLanguage?.getAttribute('hreflang');
      values.push({ value, checkLocale: !declaredLocale || declaredLocale === locale });
    };
    const walker = document.createTreeWalker(body, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      if (node.textContent.trim() && !node.parentElement.closest('script, style, template, noscript, code, pre, textarea')) {
        append(node.parentElement, node.textContent);
      }
    }
    for (const element of body.querySelectorAll('[aria-label], [alt], [title], [placeholder], input[type=submit]')) {
      for (const attribute of ['aria-label', 'alt', 'title', 'placeholder']) {
        if (element.hasAttribute(attribute)) append(element, element.getAttribute(attribute));
      }
      if (element.matches('input[type=submit]')) append(element, element.value);
    }
    return values;
  }, locale);
  const rawKeys = new Set();
  const leaks = new Set();
  let interpolation = false;
  for (const { value, checkLocale } of values) {
    const text = normalize(value);
    for (const token of text.match(/(?<![\w/-])[a-z][\w]*(?:-[\w]+)+(?![\w/-])/gu) ?? []) {
      if (keys.has(token)) rawKeys.add(token);
    }
    const leak = checkLocale && wrongLocale(text, locale);
    if (leak) leaks.add(leak);
    interpolation ||= /\{\{\s*[\w.]|\{\s*\$[\w-]+/u.test(text);
  }
  expect.soft([...rawKeys], `${target}: rendered Fluent keys`).toEqual([]);
  expect.soft([...leaks], `${target}: opposite-locale catalog markers`).toEqual([]);
  expect.soft(interpolation, `${target}: unresolved interpolation`).toBe(false);
}

function sample(route, locale) {
  return route
    .replaceAll('{lang}', locale)
    .replaceAll('{kind}', route.includes('stats/rankings') ? 'yesterday' : route.includes('rankings') ? 'experience' : 'items')
    .replaceAll('{tab}', 'experience')
    .replaceAll('{slug}', route.startsWith('/admin/pages/') ? 'rules' : 'missing-e2e')
    .replace(/\{(?:floor|zoom|x|y)\}/gu, '0')
    .replace(/\{(?:name|key|family|channel|filename|\*target|\*rest)\}/gu, 'missing-e2e')
    .replace(/\{[^}]+\}/gu, '019a541b-a000-7000-8000-000000000001');
}

function destinations(actor, locale) {
  const targets = new Map();
  for (const row of expectations) {
    if (row.method !== 'GET' || (row.actor === 'staff') !== (actor === 'staff')) continue;
    targets.set(sample(row.route, locale), row.status);
  }
  for (const row of inventory.routes) {
    const staff = /^\/(?:\{lang\}\/)?(?:admin|console)(?:\/|$)/u.test(row.route);
    if (row.methods.includes('GET') && staff === (actor === 'staff')) {
      const path = sample(row.route, locale);
      if (!targets.has(path)) targets.set(path, undefined);
    }
  }
  if (actor === 'staff') {
    for (const path of ['/admin/announcements', '/admin/announcements/create', '/admin/announcements/{id}/edit']) {
      targets.set(sample(path, locale), path.endsWith('/edit') ? 404 : 200);
    }
  }
  if (actor !== 'staff') targets.set(sample('/native-account/coin-checkout/{id}', locale), undefined);
  return targets;
}

async function addLinkedPages(page, targets, locale, actor) {
  const paths = await page.locator('a[href]:not([hreflang], [download])').evaluateAll((links) =>
    links.flatMap((link) => {
      const href = link.getAttribute('href');
      if (!URL.canParse(href, link.baseURI)) return [];
      const url = new URL(href, link.baseURI);
      // Do not replay links carrying query parameters, fragments or signed tokens.
      return url.origin === location.origin && !url.search && !url.hash ? [url.pathname] : [];
    }));
  for (const path of paths) {
    if (new RegExp(`^/(?:admin/)?${opposite(locale)}(?:/|$)`, 'u').test(path)) continue;
    if (/^\/language\/|\/(?:confirm|revert|reset-password)\/[^/]+$/u.test(path)) continue;
    const staff = /^\/(?:pl\/|en\/)?(?:admin|console)(?:\/|$)/u.test(path);
    if (staff !== (actor === 'staff') || targets.has(path)) continue;
    targets.set(path, undefined);
  }
}

const validationMessages = 'main [role=alert], main .op-error, main [aria-live]:not([aria-live=off])';

async function nativeValidation(page, locale, checked, uncovered) {
  const forms = page.locator('form');
  for (let index = 0; index < await forms.count(); index++) {
    const form = forms.nth(index);
    const { method, action, result } = await form.evaluate((form) => {
      const field = Array.from(form.elements).find((element) =>
        element.willValidate && (element.required || element.type === 'email') &&
        element.matches('input:not([type=hidden], [type=radio]), textarea, select') && element.getClientRects().length);
      const button = Array.from(form.elements).find((element) =>
        element.type === 'submit' && !element.disabled && !element.formNoValidate && element.getClientRects().length);
      // Named controls such as "action" shadow HTMLFormElement properties.
      const action = button?.getAttribute('formaction') ?? form.getAttribute('action') ?? '';
      const method = button?.getAttribute('formmethod') ?? form.getAttribute('method') ?? 'get';
      const details = { action: new URL(action || location.href, form.baseURI).pathname, method, result: null };
      if (!field || !button || form.noValidate) return details;
      const original = { value: field.value, checked: field.checked, selectedIndex: field.selectedIndex };
      if (field.type === 'checkbox') field.checked = false;
      else if (field.tagName === 'SELECT') field.selectedIndex = -1;
      else field.value = field.required ? '' : 'invalid-email';

      try {
        // Submit only if the real browser constraint already rejects the input.
        if (field.validity.valid) return details;
        let invalidEvent = false;
        field.addEventListener('invalid', () => { invalidEvent = true; }, { once: true });
        HTMLFormElement.prototype.requestSubmit.call(form, button);
        details.result = { invalidEvent, message: field.validationMessage, custom: field.validity.customError };
        return details;
      } finally {
        if (field.type === 'checkbox') field.checked = original.checked;
        else if (field.tagName === 'SELECT') field.selectedIndex = original.selectedIndex;
        else field.value = original.value;
      }
    });
    const name = `${method.toUpperCase()} ${action} (form ${index + 1})`;
    if (!result) {
      uncovered.add(name);
      continue;
    }
    expect.soft(result.invalidEvent, name).toBe(true);
    expect.soft(result.message.length, name).toBeGreaterThan(0);
    // The browser supplies native UI strings. Check authored custom messages
    // and DOM feedback against the portal's catalogs.
    if (result.custom) assertText(result.message, locale, name);
    for (const message of await page.locator(validationMessages).allInnerTexts()) {
      assertText(message, locale, name);
    }
    checked.add(name);
  }
}

function posted(page, path) {
  return page.waitForResponse((response) =>
    response.request().method() === 'POST' && new URL(response.url()).pathname === path);
}

async function serverValidation(page, locale, actor, target, targets) {
  if (actor === 'player' && target === `/${locale}/password/change`) {
    await submit(page, 'main form', {
      current_password: 'Incorrect local password!',
      password: 'Synthetic replacement password!',
      password_confirmation: 'Mismatched replacement password!',
    });
    await expect(page.locator('main [role=alert]')).toHaveText(
      catalogText(`crates/identity/src/credentials_${locale}.ftl`, 'refused'),
    );
    await assertDocument(page, locale, 'password change refusal');
  }
  if (actor === 'player' && target === `/${locale}/support/reports/create`) {
    await submit(page, 'main form', { target_reference: ' ', evidence_summary: 'Parity fixture' });
    await expect(page.locator('main [role=alert]')).toHaveText(
      catalogText(`crates/support/src/reports/${locale}.ftl`, 'invalid'),
    );
    await assertDocument(page, locale, 'report validation refusal');
  }
  if (actor === 'player' && target === `/${locale}/support/tickets/create`) {
    const response = posted(page, `/${locale}/support/tickets`);
    await submit(page, 'main form', { subject: ' ', body: 'Parity fixture' });
    expect((await response).status()).toBe(422);
    await expect(page.locator('body')).toHaveText(
      catalogText(`crates/support/src/tickets/${locale}.ftl`, 'error'),
    );
    expect((await page.goto(target)).status()).toBe(200);
    await submit(page, 'main form', { subject: 'Parity fixture', body: 'Synthetic support inquiry' });
    await expect(page).toHaveURL(/support\/tickets\/[a-f0-9-]+$/u);
    targets.set(new URL(page.url()).pathname, 200);
  }
  if (actor === 'staff' && target === '/admin/announcements/create') {
    const response = posted(page, '/admin/announcements');
    await submit(page, 'main form', { starts_at: 'invalid-date', title: 'Parity fixture', body: 'Parity fixture' });
    expect((await response).status()).toBe(422);
    await expect(page.locator('main')).toContainText(catalogText(`crates/ui/i18n/${locale}.ftl`, 'error-422-description'));
    await assertDocument(page, locale, 'announcement validation refusal');
  }
}

async function signInStaff(page, locale) {
  const account = await register(page, locale);
  await query(
    "INSERT INTO account_roles(account_id,role_id) SELECT a.id,r.id FROM accounts a CROSS JOIN roles r WHERE a.email=$1 AND r.key='platform_admin'",
    [account.email],
  );
  await login(page, locale, account);
  expect((await page.goto(`/${locale}/mfa`)).status()).toBe(200);
  await submit(page, 'main form[action$="/mfa/enroll"]', { password: account.password });
  const provisioning = await page.locator('main .op-prose').innerText();
  const totp = provisioning.startsWith('otpauth:')
    ? OTPAuth.URI.parse(provisioning)
    : new OTPAuth.TOTP({ secret: provisioning, algorithm: 'SHA1', digits: 6, period: 30 });
  await submit(page, 'main form[action$="/mfa/confirm"]', { password: account.password, code: totp.generate() });
  // The confirmation replaces the document. Wait for the recovery-code view
  // before reading it, since a count on the old DOM can return zero.
  const recovery = page.locator('textarea[readonly]').first();
  await expect(recovery).toBeVisible();
  await loginWithRecoveryCode(page, locale, account, await recovery.inputValue());
  await expect(page).toHaveURL(new RegExp(`/${locale}/$`, 'u'));
  expect((await page.goto(`/admin?lang=${locale}`)).status()).toBe(200);
}

for (const actor of ['anonymous', 'player', 'staff']) {
  test(`${actor} HTML pages use the requested language`, async ({ page }, info) => {
    test.setTimeout(600_000);
    const locale = info.project.metadata.locale;
    if (actor === 'staff') await signInStaff(page, locale);
    if (actor === 'player') await login(page, locale, await register(page, locale));
    const origin = new URL(`/${locale}/`, process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443');
    expect((await page.goto(`/language/${locale}`)).status()).toBe(200);
    const targets = destinations(actor, locale);
    const documents = new Set();
    const checked = new Set();
    const uncovered = new Set();
    for (const [target, status] of targets) {
      if (target.startsWith('/language/')) continue;
      await test.step(target, async () => {
        const url = new URL(target, origin);
        // These screens expose a locale query parameter. Other routes use
        // the language cookie or Accept-Language and can reject extra queries.
        if (/^\/admin\/(?:news(?:\/|$)|media$|console\/(?:access|approvals)$)/u.test(url.pathname)) {
          url.searchParams.set('lang', locale);
        }
        await waitForWikiBudget(target, 2);
        const probe = await page.request.get(url.href, { headers: { accept: 'text/html', 'accept-language': locale } });
        if (status !== undefined && actor !== 'anonymous') expect.soft(probe.status(), target).toBe(status);
        if (!probe.headers()['content-type']?.startsWith('text/html')) return;
        const source = await probe.text();
        // The inventory also includes HTML fragments consumed by other pages.
        if (!/<(?:!doctype\s+html|html(?:\s|>))/iu.test(source)) return;
        const final = new URL(probe.url());
        if (documents.has(final.pathname + final.search)) return;
        documents.add(final.pathname + final.search);
        const navigation = await page.goto(url.href);
        expect.soft(navigation.status(), target).toBe(probe.status());
        await assertDocument(page, locale, target);
        if (navigation.ok()) {
          await addLinkedPages(page, targets, locale, actor);
          await nativeValidation(page, locale, checked, uncovered);
          await serverValidation(page, locale, actor, target, targets);
        }
      });
    }
    expect(documents.size, `${actor}: HTML documents visited`).toBeGreaterThan(0);
    await info.attach('i18n-coverage', {
      body: JSON.stringify({ documents: [...documents].map((url) => new URL(url, origin).pathname),
        nativeValidation: [...checked], formsWithoutNativeValidation: [...uncovered] }),
      contentType: 'application/json',
    });
  });
}

test('invalid sign-in submits a localized server error', async ({ page }, info) => {
  const locale = info.project.metadata.locale;
  await login(page, locale, { email: 'i18n-missing@example.invalid', password: 'Invalid local input!' });
  await expect(page).toHaveURL(new RegExp(`/${locale}/login$`, 'u'));
  await expect(page.locator('main [role=alert]')).toHaveText(
    catalogText(`templates/account/mfa/${locale}.ftl`, 'mfa-error'),
  );
  await assertDocument(page, locale, 'sign-in refusal');
});
