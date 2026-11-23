import { expect, test } from '@playwright/test';
import { setTimeout as delay } from 'node:timers/promises';
import { ActorSessionLost, capture, crawl, save, summary } from './audit.mjs';
import {
  emailCode, fill, goto, mailLink, MailUnavailable, mailStatus, origin, password,
  protectContext, ready, submit, totp, unique,
} from './helpers.mjs';

test.beforeEach(async ({ context }) => {
  await ready();
  await protectContext(context);
});

const text = {
  pl: {
    inbox: 'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.',
    verify: 'Twój adres e-mail został zweryfikowany.',
    badRegistration: 'Popraw poniższe pola.',
    reset: 'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.',
    resetSubject: 'Zresetuj hasło do Oteryn',
    invalidName: 'Sprawdź reguły imion i wybierz prawidłowy świat, płeć oraz profesję przed ponownym wysłaniem.',
    reserved: 'To imię jest zastrzeżone. Wybierz inne.',
    unavailable: 'Tworzenie postaci jest chwilowo niedostępne.',
    pending: 'Trwa przetwarzanie prośby. Bez wyniku z gry rezultat pozostaje nieznany.',
    created: 'Gra potwierdziła utworzenie postaci.',
  },
  en: {
    inbox: 'Check your inbox for an e-mail with the next steps.',
    verify: 'Your e-mail has been verified.',
    badRegistration: 'Please correct the fields below.',
    reset: 'If an account exists for that email, a password reset link has been sent.',
    resetSubject: 'Reset your Oteryn password',
    invalidName: 'Check the name rules and choose a valid world, sex and vocation before submitting again.',
    reserved: 'This character name is reserved. Choose another name.',
    unavailable: 'Character creation is temporarily unavailable.',
    pending: 'Your request is being processed. Without a result from Game, its outcome is still unknown.',
    created: 'Game confirmed that your character was created.',
  },
};

function playerAccount() {
  return { email: `sim-visual-${unique()}-1@example.test`, password: password() };
}

async function register(page, lang, account, info, checkInvalid = false) {
  const path = `/${lang}/register`;
  expect((await goto(page, path)).status()).toBe(200);
  const form = page.locator('main form');
  const values = {
    email: account.email, password: account.password, password_confirmation: account.password,
    terms: true, privacy: true,
  };
  async function send(fields, invalid = false) {
    for (const wait of [0, 65_000, 120_000]) {
      if (wait) {
        await delay(wait);
        await ready();
        expect((await goto(page, path)).status()).toBe(200);
      }
      let challengeStatus;
      const observe = response => {
        if (new URL(response.url()).pathname === '/register/abuse/challenge') challengeStatus = response.status();
      };
      page.on('response', observe);
      let posted;
      try {
        [posted] = await Promise.all([
          page.waitForResponse(response => response.url() === origin + path
            && response.request().method() === 'POST', { timeout: 100_000 }),
          page.waitForEvent('framenavigated', {
            predicate: frame => frame === page.mainFrame(), timeout: 100_000,
          }),
          submit(form, fields),
        ]);
      } finally {
        page.off('response', observe);
      }
      const http = posted.status();
      if (http === 429 || (!invalid && http === 422 && challengeStatus === 429)) continue;
      await page.waitForLoadState('domcontentloaded');
      if (http >= 500 || (!invalid && http !== 303)) {
        const notice = await page.locator('main h1, main [role=alert], main [role=status]')
          .allTextContents();
        await save(info, 'registration-result.json', { http, challengeStatus, notice });
        await capture(page, info, 'form-states', '/registration-unavailable', { states: false });
      }
      return http;
    }
    test.skip(true, 'Registration POST or browser verification remained rate-limited after backoff');
  }
  if (checkInvalid) {
    await submit(form, { ...values, email: 'bad-address' });
    expect(await form.locator('[name=email]').evaluate(el => el.validity.typeMismatch)).toBe(true);
    await expect(page.locator('main [role=alert]')).toHaveCount(0);
    expect(await send({ ...values, password_confirmation: password() }, true)).toBe(422);
    await expect(page.locator('main [role=alert]')).toHaveText(text[lang].badRegistration);
    await expect(form.locator('[name=password_confirmation]')).toHaveAttribute('aria-invalid', 'true');
  }
  const since = Date.now() - 1_000;
  expect(await send(values)).toBe(303);
  await expect(page).toHaveURL(new RegExp(`/${lang}/register/check-inbox$`));
  await expect(page.locator('main')).toContainText(text[lang].inbox);
  return since;
}

async function login(page, lang, account) {
  await goto(page, `/${lang}/login`);
  await submit(page.locator('main form').first(), account);
  await expect(page).toHaveURL(url => url.origin === origin
    && [`/${lang}/`, `/${lang}/mfa/challenge`, '/account'].includes(url.pathname));
}

async function createPlayer(page, lang, account, info) {
  const since = await register(page, lang, account, info);
  const verify = await mailLink(account.email, since, null, /^\/(?:pl|en)\/verify-email\?token=[a-f0-9]+$/i);
  if ((await goto(page, verify)).status() !== 200) throw new Error('Verification confirmation did not open');
  await submit(page.locator('main form').first());
  await expect(page.locator('main')).toContainText(text[lang].verify);
  await login(page, lang, account);
  await expect(page.locator('[data-account-link]').first()).toBeAttached();
}

test('staff menus and bilingual editorial publication', async ({ page, context }, info) => {
  test.skip(info.project.name !== 'en-1440', 'One run owns the two shared staff accounts across all six locale/viewports');
  // A single owner prevents concurrent password resets on these shared actors.
  // Do not send reset mail during a known Mailpit outage.
  const deadline = Date.now() + 90 * 60_000;
  test.setTimeout(110 * 60_000);
  const results = [];
  const egressBlockers = [];
  const recoveryDefects = [];
  const probes = [];
  for (const actor of ['content-editor', 'moderator']) {
    for (let attempt = 0; attempt < 2; attempt++) {
      const staff = await context.newPage();
      try {
        let blocker;
        do {
          const http = await mailStatus();
          probes.push({ checkedAt: new Date().toISOString(), actor, http });
          await save(info, 'mail-availability.json', probes);
          blocker = [404, 405].includes(http) ? `Mailpit HTTP ${http}` : undefined;
          if (!blocker) {
            expect(http).toBe(200);
            try {
              await staffLogin(staff, actor, info);
            } catch (error) {
              if (!(error instanceof MailUnavailable)) throw error;
              blocker = error.message;
            }
          }
          if (!blocker || Date.now() >= deadline) break;
          await delay(Math.min(600_000, deadline - Date.now()));
          await ready();
        } while (true);
        test.skip(Boolean(blocker), `Staff login blocked: ${blocker}`);
        const slices = [];
        for (const language of ['en', 'pl']) {
          // Some older admin handlers select their language from Accept-Language.
          const localeContext = await context.browser().newContext({
            baseURL: origin, serviceWorkers: 'block', proxy: info.project.use.proxy,
            ignoreHTTPSErrors: info.project.use.ignoreHTTPSErrors, locale: language === 'pl' ? 'pl-PL' : 'en-GB',
          });
          try {
            await protectContext(localeContext);
            await localeContext.addCookies(await context.cookies(origin));
            const localized = await localeContext.newPage();
            for (const width of [1440, 768, 390]) slices.push(await crawl(localized, info, actor, language, width));
          } finally {
            await localeContext.close();
          }
        }
        if (actor === 'content-editor') await editorial(staff, page, info);
        results.push(...slices);
        break;
      } catch (error) {
        if (error instanceof StaffRecoveryFailure) {
          recoveryDefects.push(`${actor}: ${error.message}`);
          break;
        }
        if (error instanceof SharedEgress) {
          egressBlockers.push(`${actor}: ${error.message}`);
          break;
        }
        if (!(error instanceof ActorSessionLost) || attempt) throw error;
        info.annotations.push({ type: 'redeploy retry', description: error.message });
        await ready();
      } finally {
        await staff.close();
        await context.clearCookies();
      }
    }
  }
  await save(info, 'staff-scenario.json', { recoveryDefects, egressBlockers });
  expect([...recoveryDefects, ...summary(results)]).toEqual([]);
  test.skip(egressBlockers.length > 0, egressBlockers.join(', '));
});


test('registration shows inline validation and confirms the inbox handoff', async ({ page }, info) => {
  const lang = info.project.metadata.language;
  await register(page, lang, playerAccount(), info, true);
  await page.reload({ waitUntil: 'domcontentloaded' });
  await expect(page.locator('main')).toContainText(text[lang].inbox);
  await capture(page, info, 'form-states', '/registration-inbox');
});

test('verified player and character views stay consistent', async ({ page, context }, info) => {
  const lang = info.project.metadata.language;
  const width = info.project.use.viewport.width;
  const mail = await mailStatus();
  test.skip([404, 405].includes(mail), `Email verification blocked: Mailpit HTTP ${mail}`);
  expect(mail).toBe(200);
  for (let attempt = 0; attempt < 2; attempt++) {
    try {
      await createPlayer(page, lang, playerAccount(), info);
      const results = [await crawl(page, info, 'player', lang, width)];
      if (width === 1440) results.push(await crawl(page, info, 'player', lang, 768));
      const character = await createCharacter(page, info, lang);
      if (character.state === 'created') {
        results.push(await crawl(page, info, 'player-with-character', lang, width));
        if (width === 1440) results.push(await crawl(page, info, 'player-with-character', lang, 768));
      }
      await save(info, 'entities.json', character.matrix);
      expect(summary(results)).toEqual([]);
      return;
    } catch (error) {
      if (!(error instanceof ActorSessionLost) || attempt) throw error;
      info.annotations.push({ type: 'redeploy retry', description: error.message });
      await ready();
      await context.clearCookies();
    }
  }
});

async function createCharacter(page, info, lang) {
  const path = `/${lang}/account/characters/new`;
  const response = await goto(page, path);
  const posted = page.locator('main form[method=post]');
  const status = page.getByRole('status');
  if (response.status() === 404) {
    expect(await page.locator(`a[href="${path}"]`).count()).toBe(0);
    return { state: 'unmounted', matrix: { character: 'Character creation route unmounted' } };
  }
  expect([200, 503]).toContain(response.status());
  if (!await posted.count()) {
    await expect(status).toHaveText(text[lang].unavailable);
    return { state: 'unavailable', matrix: { character: 'Game unavailable (localized notice)' } };
  }
  const letters = unique().replace(/\d/g, v => String.fromCharCode(103 + Number(v)));
  const name = `Visual ${letters[0].toUpperCase()}${letters.slice(1)}`;
  await posted.locator('[name=world] option[value]:not([value=""])').first().waitFor({ state: 'attached' });
  const world = await posted.locator('[name=world] option[value]:not([value=""])').first().getAttribute('value');
  async function send(value) {
    await posted.locator('[name=vocation]').first().check();
    await posted.locator('[name=sex]').first().check();
    await submit(posted, { name: value, world });
  }
  await send('1');
  await expect(status).toHaveText(text[lang].invalidName);
  await send('Admin');
  await expect(status).toHaveText(text[lang].reserved);
  await send(name);
  await expect(page).toHaveURL(/\/account\/characters\/requests\/[^/]+$/);
  await expect(status).toBeVisible();
  await page.reload({ waitUntil: 'domcontentloaded' });
  const result = await status.innerText();
  if (result.includes(text[lang].pending) || result === text[lang].unavailable) {
    await capture(page, info, 'character', '/request-pending');
    return { state: 'pending', matrix: { character: 'Request accepted; no completed Game projection' } };
  }
  await expect(status).toContainText(text[lang].created);
  expect((await goto(page, '/account')).status()).toBe(200);
  await expect(page.locator('main')).toContainText(name, { timeout: 20_000 });
  await goto(page, `/${lang}/characters`);
  await submit(page.locator('main form').first(), { name });
  await expect(page.locator('main h2').filter({ hasText: name })).toBeVisible();
  const levelValue = await page.locator('main dl dd').first().innerText();
  expect(levelValue).toMatch(/^\d+$/);
  return { state: 'created', matrix: { character: 'Account overview and public character search both show the new name',
    level: 'Public character search shows a numeric level' } };
}

class SharedEgress extends Error {}
class StaffRecoveryFailure extends Error {}

async function recoveryPost(page, screen, target, values) {
  for (const wait of [0, 65_000, 120_000]) {
    if (wait) {
      await delay(wait);
      await ready();
    }
    const loaded = await goto(page, screen);
    if (loaded.status() !== 200) throw new Error(`Staff recovery form HTTP ${loaded.status()}`);
    let posted;
    try {
      [posted] = await Promise.all([
        page.waitForResponse(response => new URL(response.url()).pathname === target
          && response.request().method() === 'POST'),
        submit(page.locator('main form'), values),
      ]);
    } catch {
      throw new Error('Staff could not submit the standard recovery form');
    }
    if (posted.status() !== 429) return posted;
  }
  throw new SharedEgress(`${target} POST remained HTTP 429 after backoff`);
}

async function staffLogin(page, role, info) {
  const lang = 'en';
  const account = { email: `staff-${role}@example.invalid`, password: password() };
  const since = Date.now() - 1_000;
  const forgot = await recoveryPost(page, '/en/forgot-password', '/en/forgot-password', { email: account.email });
  expect(forgot.status()).toBe(302);
  await expect(page.locator('main')).toContainText(text.en.reset);
  const reset = await mailLink(account.email, since, text.en.resetSubject, /^\/(?:en|pl)\/reset-password\/[a-f0-9]+$/i);
  const completed = await recoveryPost(page, reset, '/en/reset-password', { ...account, password_confirmation: account.password });
  const location = completed.headers().location;
  if (completed.status() !== 302 || !location || new URL(location, origin).pathname !== '/en/login') {
    if (completed.status() >= 500) {
      await page.waitForLoadState('domcontentloaded');
      await capture(page, info, 'staff-recovery', `/${role}-reset-unavailable`, { states: false });
    }
    throw new StaffRecoveryFailure(`valid password reset did not advance to sign-in (HTTP ${completed.status()})`);
  }
  await page.waitForURL('**/en/login');
  await page.waitForLoadState('domcontentloaded');
  await login(page, lang, account);
  if (new URL(page.url()).pathname.endsWith('/mfa/challenge')) {
    const requested = Date.now() - 1_000;
    await submit(page.locator('form[action$="/mfa/email"]'));
    const fallbackCode = await emailCode(account.email, requested);
    try {
      await submit(page.locator('form[action$="/mfa/challenge"]'), { code: fallbackCode });
      await expect(page).not.toHaveURL(/mfa\/challenge/);
    } catch {
      throw new Error('Staff could not complete the standard email challenge');
    }
    expect((await goto(page, '/admin')).status()).toBe(200);
    return;
  }
  await goto(page, '/en/mfa');
  await submit(page.locator('form[action$="/mfa/enroll"]'), { password: account.password });
  try {
    const provisioning = (await page.locator('main .op-prose').innerText()).trim();
    if (Date.now() % 30_000 > 25_000) await delay(5_000);
    await submit(page.locator('form[action$="/mfa/confirm"]'), {
      password: account.password, code: totp(provisioning),
    });
    await expect(page.locator('textarea[readonly]').first()).toBeVisible();
    await login(page, lang, account);
    if (new URL(page.url()).pathname.endsWith('/mfa/challenge')) {
      if (Date.now() % 30_000 > 25_000) await delay(5_000);
      await submit(page.locator('form[action$="/mfa/challenge"]'), { code: totp(provisioning) });
    }
  } catch {
    throw new Error('Staff could not complete the standard authenticator enrollment');
  }
  expect((await goto(page, '/admin')).status()).toBe(200);
}


async function editorial(staff, reader, info) {
  const path = '/admin/news/create?lang=en';
  const slug = `visual-${unique()}`;
  const time = new Date(Date.now() - 60_000).toISOString();
  const date = time.slice(0, 10);
  const titles = { en: 'Staging visual audit', pl: 'Audyt wyglądu portalu testowego' };
  const body = { en: 'The current visual check uses this test entry.', pl: 'Tego wpisu używa bieżąca kontrola wyglądu.' };
  const input = { slug, title: titles.en, title_pl: titles.pl, body: body.en,
    body_pl: body.pl, published_at: time, translation_published_at: time };
  await goto(staff, path);
  const form = staff.locator('main form').first();
  const bad = staff.waitForResponse(response => response.url() === origin + '/admin/news?lang=en'
    && response.request().method() === 'POST');
  await submit(form, { ...input, title: '' });
  expect((await bad).status()).toBe(422);
  await capture(staff, info, 'editorial', '/invalid-input');
  await expect.soft(staff.locator('main [role=alert]'), 'An editorial validation refusal explains what to fix')
    .toBeVisible({ timeout: 2_000 });
  await goto(staff, path);
  await submit(form, { ...input, published_at: '', translation_published_at: '' });
  await expect(staff).toHaveURL(/\/admin\/news\/[^/]+\/edit/);
  const edit = new URL(staff.url()).pathname + '?lang=en';
  const article = lang => `/${lang}/news/${slug}`;
  const anonymous = await reader.context().browser().newContext({ baseURL: origin, serviceWorkers: 'block',
    proxy: info.project.use.proxy, ignoreHTTPSErrors: info.project.use.ignoreHTTPSErrors });
  await protectContext(anonymous);
  const publicPage = await anonymous.newPage();
  try {
    expect((await goto(publicPage, article('en'))).status()).toBe(404); // Draft is hidden.
    await goto(staff, edit);
    await fill(form, input);
    await submit(form);
    await expect(form.locator('[name=title]')).toHaveValue(titles.en);
    await staff.reload({ waitUntil: 'domcontentloaded' });
    await expect(form.locator('[name=body_pl]')).toHaveValue(body.pl);
    const matrix = [];
    for (const lang of ['en', 'pl']) {
      expect((await goto(publicPage, article(lang))).status()).toBe(200);
      await expect(publicPage.locator('main h1')).toHaveText(titles[lang]);
      await expect(publicPage.locator('main article')).toHaveText(body[lang]);
      const detailTime = await publicPage.locator('main time').allTextContents();
      await goto(publicPage, `/${lang}/news`);
      await expect(publicPage.locator(`main a[href="${article(lang)}"]`)).toHaveText(titles[lang]);
      const listTime = await publicPage.locator(`main a[href="${article(lang)}"]`).locator('..').locator('time').allTextContents();
      await goto(publicPage, `/${lang}/`);
      const story = publicPage.locator('.hero-story').filter({ hasText: titles[lang] });
      await expect(story.locator('.hero-story-title')).toHaveText(titles[lang]);
      await expect(story.locator('time')).toHaveAttribute('datetime', date);
      await story.locator('summary').click();
      await expect(story).toContainText(body[lang]);
      matrix.push({ locale: lang, edit: 'title/body persisted', list: 'title present',
        detail: 'title/body match', home: 'title/date/body match', listTime, detailTime });
    }
    await save(info, 'editorial-matrix.json', matrix);
  } finally {
    await anonymous.close();
    await goto(staff, edit);
    await submit(staff.locator('main form[action$="/delete"]'));
  }
}
