import { randomBytes } from 'node:crypto';
import { readFileSync } from 'node:fs';
import {
  at, enableAccess, expect, localized, mail, mailLink, mainForm, open, origin,
  password, player, sharedLimit, submit, test, withBudget,
} from './helpers.mjs';

function translations(path, lang) {
  const lines = readFileSync(new URL(`../../../../${path.replaceAll('{lang}', lang)}`, import.meta.url), 'utf8');
  return Object.fromEntries([...lines.matchAll(/^([\w-]+) = (.+)$/gm)].map(([, key, value]) => [key, value]));
}

function copy(lang) {
  return {
    ...translations('crates/ui/i18n/{lang}.ftl', lang),
    ...translations('templates/account/mfa/{lang}.ftl', lang),
    ...translations('templates/account/reset/{lang}.ftl', lang),
    breach: translations('crates/identity/src/abuse/{lang}.ftl', lang)['breach-error'],
    account: translations('templates/account/overview.{lang}.ftl', lang),
    bazaar: translations('templates/bazaar/{lang}.ftl', lang),
    character: translations('crates/characters/src/create_web/{lang}.ftl', lang),
    preferences: translations('crates/identity/src/account_lifecycle_{lang}.ftl', lang),
  };
}

function registrationFields(account) {
  return { ...account, password_confirmation: account.password, terms: true, privacy: true };
}

async function register(page, lang, info, fields, options) {
  return withBudget(info, async () => {
    expect((await open(page, `/${lang}/register`)).status()).toBe(200);
    const responses = [];
    const observeChallenge = response => {
      if (new URL(response.url()).pathname === '/register/abuse/challenge') responses.push(response);
    };
    page.on('response', observeChallenge);
    try {
      const response = await submit(page, mainForm(page, '/register'), fields, options);
      return { response, limit: responses.find(item => item.status() === 429) ?? response };
    } finally {
      page.off('response', observeChallenge);
    }
  }, result => result.limit).then(result => result.response);
}

async function inbox(page, lang, text, response) {
  expect(response.status()).toBe(303);
  await expect(page.locator('main')).toContainText(text['verify-inbox']);
  at(page, `/${lang}/register/check-inbox`);
  await localized(page, lang);
}

async function login(page, lang, info, account, allowed = true) {
  const result = await withBudget(info, async () => {
    expect((await open(page, `/${lang}/login`)).status()).toBe(200);
    return submit(page, mainForm(page, `/${lang}/login`), account);
  });
  expect(result.status()).toBe(302);
  if (allowed) {
    await expect(page.locator('[data-account-link]').first()).toHaveCount(1);
    at(page, `/${lang}/`);
  } else {
    await expect(page.locator('main [role=alert]')).toHaveText(copy(lang)['mfa-error']);
    at(page, `/${lang}/login`);
  }
}

async function accountHome(page, lang) {
  const response = await open(page, '/account');
  expect(response.status()).toBe(200);
  await localized(page, lang);
  const text = copy(lang).account;
  await expect(page.getByRole('heading', { name: text.overview, exact: true })).toBeVisible();
  await expect(page.locator('main')).toContainText(text.active);
}

async function accountMenu(page, lang) {
  expect((await open(page, `/${lang}/`)).status()).toBe(200);
  const toggle = page.locator('#portal-menu-toggle');
  if (await toggle.isVisible()) await toggle.click();
  await page.locator('[data-account-link]').click();
  const menu = page.getByRole('dialog');
  await expect(menu).toBeVisible();
  return menu;
}

async function openVerification(page, info, link) {
  return withBudget(info, () => open(page, link));
}

async function expectInvalidVerification(page, info, lang, link) {
  // Keep credential-bearing paths and DOM fragments out of failure diagnostics.
  const response = await openVerification(page, info, link);
  expect(response.status()).toBe(422);
  await localized(page, lang);
  await expect.poll(async () => (await page.locator('main').innerText()).includes(copy(lang)['verify-invalid'])).toBe(true);
}

async function accountPages(page, lang, info) {
  const text = copy(lang);
  const menu = await accountMenu(page, lang);
  const links = await menu.getByRole('link').evaluateAll(anchors => anchors.map(a => a.href));
  expect(links).toContain(`${origin}/account`);
  expect(links).toContain(`${origin}/${lang}/account/sessions`);
  const unavailable = new Set(['account/journal/reports', 'account/bazaar']);
  const disabled = new Set(['panel/premium', 'shop/history', 'panel/payments']);
  const settings = ['account/security', 'account/security/email', 'account/sudo', 'mfa', ...disabled];
  for (const href of new Set([...links, ...settings.map(path => `${origin}/${lang}/${path}`)])) {
    const url = new URL(href);
    if (url.origin !== origin) {
      expect(url.protocol).toBe('https:');
      continue;
    }
    await test.step(url.pathname, async () => {
      const response = await open(page, url);
      const target = url.pathname.slice(lang.length + 2);
      const expected = unavailable.has(target) ? 503 : disabled.has(target) ? 404 : 200;
      expect.soft(response.status(), url.pathname).toBe(expected);
      await expect.soft(page.locator('main')).toBeVisible();
      await expect.soft(page.locator('html')).toHaveAttribute('lang', lang);
      if (target === 'account/bazaar') {
        await expect.soft(page.locator('main')).toContainText(text.bazaar['authority-unavailable']);
      } else if (expected !== 200) {
        await expect.soft(page.locator('main')).toContainText(text[`error-${expected}-title`]);
      }
      expect.soft(new URL(page.url()).pathname, url.pathname).not.toBe(`/${lang}/login`);
    });
  }

  await accountHome(page, lang);
  const section = page.getByRole('region', { name: text.account.characters });
  await expect(section).toBeVisible();
  const overviewPaths = await page.locator('main a[href]').evaluateAll(anchors => anchors
    .map(a => new URL(a.href)).filter(url => url.origin === location.origin).map(url => url.pathname));
  if (overviewPaths.some(path => !path.startsWith(`/${lang}/`))) {
    await page.locator('main').screenshot({ path: info.outputPath('overview-links.png') });
  }
  expect.soft(overviewPaths.filter(path => !path.startsWith(`/${lang}/`)),
    'Each local account overview link keeps the active language').toEqual([]);

  const target = `/${lang}/account/characters/new`;
  const creation = await open(page, target);
  if (creation.status() === 404) {
    // The staging deploy leaves the web creation and Game command flags off.
    await expect(page.locator('main')).toContainText(text['error-404-title']);
    await accountHome(page, lang);
    await expect(section.getByRole('status')).toHaveText(text.account.unavailable);
    await expect(section.locator('li')).toHaveCount(0);
    expect((await open(page, `/${lang}/`)).status()).toBe(200);
    const guide = page.locator('#jak-zaczac');
    await expect(guide).toContainText(text['thais-home-character-unavailable']);
    await expect(guide.locator(`[href="${target}"]`)).toHaveCount(0);
    info.annotations.push({ type: 'staging limitation', description: 'Character creation route unmounted. Name entry requires Game.' });
    return;
  }
  const form = mainForm(page, target);
  if (creation.status() === 503) {
    await expect(page.locator('main')).toContainText(text['error-503-title']);
    info.annotations.push({ type: 'staging limitation', description: 'Character creation has no available Game world.' });
    return;
  }
  expect(creation.status()).toBe(200);
  if (!await form.count()) {
    await expect(page.locator('main')).toContainText(text.character.unavailable);
    info.annotations.push({ type: 'staging limitation', description: 'Character creation is unavailable.' });
    return;
  }
  await expect(form).toBeVisible();
  const world = await form.locator('select[name=world] option').nth(1).getAttribute('value');
  expect(Boolean(world)).toBe(true);
  async function sendName(name) {
    await form.locator('select[name=world]').selectOption(world);
    await form.locator('input[name=sex]').first().check();
    await form.locator('input[name=vocation]').first().check();
    return submit(page, form, { name });
  }
  for (const name of ['A'.repeat(80), 'Żółty Rycerz', '   ']) {
    expect((await sendName(name)).status()).toBe(302);
    await expect(page.locator('main')).toContainText(text.character['name-invalid']);
  }
  const suffix = randomBytes(6).toString('hex').replace(/[0-9]/g, digit => String.fromCharCode(103 + Number(digit)));
  expect((await sendName(`Sima ${suffix}`)).status()).toBe(302);
  await expect(page).toHaveURL(new RegExp(`/${lang}/account/characters/requests/[a-f0-9-]+$`));
  await expect(page.locator('[role=status]')).toHaveText(text.character.pending);
  expect((await page.reload()).status()).toBe(200);
  await expect(page.locator('[role=status]')).toHaveText(text.character.pending);
}

test('registration validates address, password, confirmation and both consents', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const text = copy(lang);
  const account = player();
  const form = mainForm(page, '/register');

  expect((await open(page, `/${lang}/register`)).status()).toBe(200);
  await expect(page.getByRole('heading', { name: text['register-title'] })).toBeVisible();
  await localized(page, lang);
  await expect(form.locator('[name=name]')).toHaveCount(0);
  await form.locator('[name=email]').fill('missing-at-sign');
  await form.locator('[name=password]').fill('short');
  await form.locator('[name=password_confirmation]').fill('mismatch');
  await form.locator('button[type=submit]').click();
  expect(await form.locator('[name=email]').evaluate(input => input.validity.typeMismatch)).toBe(true);
  for (const consent of ['terms', 'privacy']) {
    expect(await form.locator(`[name=${consent}]`).evaluate(input => input.validity.valueMissing)).toBe(true);
  }
  at(page, `/${lang}/register`);

  const cases = [
    {
      values: { email: 'missing-at-sign', password: 'short', password_confirmation: 'mismatch', terms: false, privacy: false },
      errors: {
        '#email-error': 'register-email-error',
        '#password-error': 'register-password-error',
        '#confirmation-error': 'register-confirm-error',
        '#terms-error': 'register-consent-error',
        '#privacy-error': 'register-consent-error',
      },
    },
    {
      values: { password: 'ż'.repeat(129), password_confirmation: 'ż'.repeat(129) },
      errors: { '#password-error': 'register-password-error' },
    },
  ];
  for (const { values, errors } of cases) {
    const response = await register(page, lang, info, { ...registrationFields(account), ...values }, { nativeValidation: false });
    expect(response.status()).toBe(422);
    await expect(page.locator('main [role=alert]')).toHaveText(text['register-errors']);
    for (const [selector, key] of Object.entries(errors)) {
      await expect(page.locator(selector)).toHaveText(text[key]);
    }
  }
});

test('both staging sample passwords get the localized breached-password refusal', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  for (const sample of [1, 2]) {
    await test.step(`Sample ${sample}`, async () => {
      const account = { ...player(), password: `Staging-Breached-Password-${sample}` };
      const response = await register(page, lang, info, registrationFields(account));
      expect.soft(response.status()).toBe(422);
      await expect.soft(page.locator('main')).toContainText(copy(lang).breach);
      await localized(page, lang);
    });
  }
});

test('new player verifies mail, signs in, manages the account and resets the password', async ({ page, browser, request }, info) => {
  const { lang } = info.project.metadata;
  const text = copy(lang);
  const account = player();
  const makeContext = async width => {
    const options = { locale: lang, viewport: { width, height: width === 390 ? 844 : 960 }, baseURL: origin,
      serviceWorkers: 'block', proxy: info.project.use.proxy, ignoreHTTPSErrors: info.project.use.ignoreHTTPSErrors };
    const other = await browser.newContext(options);
    await enableAccess(other);
    return other;
  };
  const phoneContext = await makeContext(390);
  const resetContext = await makeContext(info.project.use.viewport.width);
  try {
    const phone = await phoneContext.newPage();
    const recovery = await resetContext.newPage();

    await test.step('Registration, duplicate inbox and verification mail', async () => {
      const response = await register(page, lang, info, registrationFields(account), { impatient: true });
      await inbox(page, lang, text, response);
      const message = await mail(request, account.email, lang === 'pl' ? 'Potwierdź adres e-mail' : 'Verify your email address');
      const link = mailLink(message, new RegExp(`^/${lang}/verify-email$`));

      await login(page, lang, info, account, false);
      const duplicate = await register(phone, lang, info, registrationFields({ ...account, password: password() }));
      await inbox(phone, lang, text, duplicate);
      const notice = await mail(request, account.email, lang === 'pl' ? 'Konto już istnieje' : 'Account already exists');
      expect(Boolean(mailLink(notice, new RegExp(`^/${lang}/login$`)))).toBe(true);

      const altered = new URL(link);
      const original = altered.searchParams.get('token');
      expect(Boolean(original && /^[a-f0-9]{64}$/i.test(original))).toBe(true);
      altered.searchParams.set('token', `${original[0] === '0' ? '1' : '0'}${original.slice(1)}`);
      await expectInvalidVerification(page, info, lang, altered);

      expect((await openVerification(page, info, link)).status()).toBe(200);
      await expect(page.getByRole('heading', { name: text['verify-title'] })).toBeVisible();
      const verified = await withBudget(info, async () => {
        if (new URL(page.url()).search !== link.search || await mainForm(page, '/verify-email').count() === 0) {
          expect((await openVerification(page, info, link)).status()).toBe(200);
        }
        return submit(page, mainForm(page, '/verify-email'));
      });
      expect(verified.status()).toBe(200);
      await expect(page.locator('main')).toContainText(text['verify-success']);
      await expectInvalidVerification(phone, info, lang, link);
      const unknown = new URL(link);
      unknown.searchParams.set('token', randomBytes(32).toString('hex'));
      await expectInvalidVerification(phone, info, lang, unknown);
    });

    await test.step('Two browser sessions, localized overview and persisted preferences', async () => {
      await login(page, lang, info, account);
      await accountHome(page, lang);
      await login(phone, lang, info, account);
      await accountHome(phone, lang);

      const path = `/${lang}/account/notifications`;
      expect((await open(page, path)).status()).toBe(200);
      await expect(page.getByRole('heading', { name: text.preferences.preferences })).toBeVisible();
      await expect(page.locator('main')).toContainText(text.preferences.security);
      const form = page.locator('main form[method="post"]');
      const preference = form.locator('[name=news_portal]');
      const next = !await preference.isChecked();
      expect((await submit(page, form, { news_portal: next })).status()).toBe(303);
      expect((await open(phone, path)).status()).toBe(200);
      await expect(phone.locator('[name=news_portal]')).toBeChecked({ checked: next });
      expect((await page.reload({ waitUntil: 'domcontentloaded' })).status()).toBe(200);
      await expect(preference).toBeChecked({ checked: next });

      const logout = (await accountMenu(phone, lang)).locator('form[action$="/logout"]');
      expect((await submit(phone, logout)).status()).toBe(302);
      expect((await open(phone, '/account')).status()).toBe(200);
      at(phone, `/${lang}/login`);
      await accountHome(page, lang);
      await login(phone, lang, info, account);
      await accountHome(phone, lang);
    });

    await test.step('Account navigation, language switch and character availability', async () => {
      const alternate = lang === 'pl' ? 'en' : 'pl';
      expect((await open(page, `/${lang}/`)).status()).toBe(200);
      const switcher = page.locator(`footer a[hreflang="${alternate}"]`);
      const href = new URL(await switcher.getAttribute('href'), origin);
      expect(href.origin).toBe(origin);
      await switcher.click();
      await localized(page, alternate);
      expect((await page.goBack()).status()).toBe(200);
      await localized(page, lang);
      await accountPages(page, lang, info);
    }).catch(error => { expect.soft(false, error.message).toBe(true); });

    await test.step('Reset through mail, old credentials rejected and both sessions revoked', async () => {
      const forgot = `/${lang}/forgot-password`;
      const requestReset = async () => {
        expect((await open(recovery, forgot)).status()).toBe(200);
        return submit(recovery, mainForm(recovery, '/forgot-password'), { email: account.email });
      };
      expect((await withBudget(info, requestReset)).status()).toBe(302);
      await expect(recovery.locator('main [role=status]')).toHaveText(text['reset-notice']);
      const link = mailLink(await mail(request, account.email, text['reset-mail-subject']),
        new RegExp(`^/${lang}/reset-password(?:/[a-f0-9]{64})?$`, 'i'));
      const next = password();
      const setPassword = async confirmation => withBudget(info, async () => {
        expect((await open(recovery, link)).status()).toBe(200);
        return submit(recovery, mainForm(recovery, '/reset-password'),
          { email: account.email, password: next, password_confirmation: confirmation });
      });
      expect((await setPassword(password())).status()).toBe(302);
      await expect.poll(async () => (await recovery.locator('main').innerText()).includes(text['reset-error'])).toBe(true);
      expect((await setPassword(next)).status()).toBe(302);
      await expect(mainForm(recovery, `/${lang}/login`)).toBeVisible();
      at(recovery, `/${lang}/login`);

      for (const session of [page, phone]) {
        expect((await open(session, '/account')).status()).toBe(200);
        await expect(mainForm(session, `/${lang}/login`)).toBeVisible();
        at(session, `/${lang}/login`);
      }
      await login(recovery, lang, info, account, false);
      await login(recovery, lang, info, { ...account, password: next });
      await accountHome(recovery, lang);
    });
  } finally {
    await Promise.all([phoneContext.close(), resetContext.close()]);
  }
});

test('password recovery shows a localized wait page after repeated requests', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const text = copy(lang);
  const account = player();
  const target = `/${lang}/forgot-password`;
  let minute;
  let submitted = 0;
  for (let attempt = 0; attempt < 12; attempt++) {
    expect((await open(page, target)).status()).toBe(200);
    const response = await submit(page, mainForm(page, '/forgot-password'), { email: account.email });
    const window = Math.floor(Date.parse(response.headers().date) / 60_000);
    expect(Number.isFinite(window), 'The edge response includes the server time').toBe(true);
    if (window !== minute) {
      minute = window;
      submitted = 0;
    }
    if (response.status() === 429) {
      if (submitted < 3) {
        await sharedLimit(response, info);
        continue;
      }
      const wait = Number(response.headers()['retry-after']);
      expect(wait).toBeGreaterThan(0);
      expect(wait).toBeLessThanOrEqual(60);
      await localized(page, lang);
      await expect(page.locator('main')).toContainText(text['error-429-title']);
      await expect(page.locator('main')).toContainText(text['error-429-description']);
      return;
    }
    expect(response.status()).toBe(302);
    await expect(page.locator('main [role=status]')).toHaveText(text['reset-notice']);
    submitted++;
    expect(submitted, 'Password recovery accepts at most three requests per address and source per minute').toBeLessThanOrEqual(3);
  }
  throw new Error('Shared staging traffic prevented reaching the per-address limit in one server minute');
});
