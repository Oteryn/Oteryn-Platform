import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import * as OTPAuth from 'otpauth';
import { expect } from '@playwright/test';
import { action, loginWithRecoveryCode, membership, query } from './staff-rbac-helpers.mjs';
import { login, password, submit } from './helpers.mjs';
import { fixtureProcessEnv } from './staff-fixture-database.mjs';

export function actorContext(browser, info) {
  return browser.newContext({
    baseURL: info.project.use.baseURL,
    ignoreHTTPSErrors: true,
    locale: info.project.metadata.locale,
    viewport: info.project.use.viewport,
  });
}

export function portalCommand(args) {
  const binary = process.env.PORTAL_E2E_BINARY
    || new URL('../target/debug/oteryn-portal', import.meta.url).pathname;
  return spawnSync(binary, args, {
    env: fixtureProcessEnv(),
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
  });
}

export async function signInWithMfa(page, lang, credentials, checkInvalidPassword = false) {
  await login(page, lang, credentials);
  await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
  const protectedPage = await page.request.get('/admin', { headers: { 'Accept-Language': lang } });
  expect(protectedPage.status()).toBe(403);
  const required = lang === 'pl'
    ? 'Wymagane uwierzytelnianie wieloskładnikowe' : 'Multi-factor authentication required';
  expect((await protectedPage.text()).includes(required), required).toBe(true);
  await open(page, `/${lang}/mfa`);
  const enrollment = 'main form[action$="/mfa/enroll"]';
  if (checkInvalidPassword) {
    await submit(page, enrollment, { password: 'Invalid fixture password!' });
    await expect(page.locator('main [role=alert]')).toContainText(lang === 'pl'
      ? 'Nie udało się zweryfikować żądania.' : 'The request could not be verified.');
  }
  await submit(page, enrollment, { password: credentials.password });
  const confirmation = 'main form[action$="/mfa/confirm"]';
  await expect(page.locator(confirmation)).toBeVisible();
  const authenticator = new OTPAuth.TOTP({
    secret: await page.locator('main .op-prose').innerText(), algorithm: 'SHA1', digits: 6, period: 30,
  });
  await submit(page, confirmation, { password: credentials.password, code: authenticator.generate() });
  const recovery = page.locator('main textarea[readonly]').first();
  await expect(recovery).toBeVisible();
  const codes = await page.locator('main textarea[readonly]').allTextContents();
  expect(codes.length).toBeGreaterThan(1);
  await loginWithRecoveryCode(page, lang, credentials, codes[0]);
  await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
  return codes[1];
}

export async function staffActor(browser, info, admin, roles, contexts) {
  const context = await actorContext(browser, info);
  contexts.push(context);
  const page = await context.newPage();
  const lang = info.project.metadata.locale;
  const credentials = { email: `staff-${randomUUID()}@example.invalid`, password: password() };
  const seeded = portalCommand(['seed', 'account', JSON.stringify({ ...credentials, locale: lang })]);
  expect(seeded.status, 'the harness creates a verified fixture account').toBe(0);
  const { accountId: id } = JSON.parse(seeded.stdout);
  if (admin) {
    for (const role of roles) await membership(admin, id, role, 'grant');
  } else {
    // Same disposable first-staff fixture as isolatedAdmin and admin.spec.
    await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'", [id]);
  }
  return { id, page, credentials };
}

export async function open(page, path) {
  expect((await page.goto(path, { waitUntil: 'domcontentloaded' })).status(), path).toBe(200);
}

export async function responseForForm(page, locator, values, identity = page, button) {
  const form = page.locator(locator).first();
  const target = new URL(await form.getAttribute('action') || page.url(), page.url());
  const fields = await form.evaluate(node => Object.fromEntries(new FormData(node)));
  if (button) fields.action = button;
  if (identity !== page) {
    delete fields._csrf;
    const token = await identity.locator('input[name="_csrf"]').first().inputValue();
    fields._csrf = token;
  }
  return identity.request.post(target.href, {
    form: { ...fields, ...values },
    headers: { origin: target.origin },
    maxRedirects: 0,
  });
}

export async function denied(page, locator, values, player, anonymous, button) {
  const wrong = await responseForForm(page, locator, values, player, button);
  expect(wrong.status()).toBe(403);
  const locale = await page.locator('html').getAttribute('lang');
  await open(anonymous, `/${locale}/login`);
  const guest = await responseForForm(page, locator, values, anonymous, button);
  expect(guest.status()).toBe(303);
  expect(guest.headers().location).toBe('/login');
}

export async function publishEditorial(editor, visitor, player, lang, evidence) {
  const key = `staff-${randomUUID()}`;
  const now = Date.now();
  const date = new Date(now - 60_000).toISOString();
  const translated = {
    en: { news: 'Journey dispatch', event: 'Journey gathering', wiki: 'Journey guide', body: 'Published by the staff journey.' },
    pl: { news: 'Wiadomości z wyprawy', event: 'Spotkanie podróżników', wiki: 'Poradnik wyprawy', body: 'Treść opublikowana przez redakcję.' },
  };
  const { en, pl } = translated;

  await open(editor, '/admin/news');
  await editor.locator('main a[href^="/admin/news/create"]').click();
  const news = {
    slug: key, title: en.news, body: en.body, published_at: date,
    title_pl: pl.news, body_pl: pl.body, translation_published_at: date,
  };
  await denied(editor, 'main form', news, player, visitor);
  expect((await responseForForm(editor, 'main form', { ...news, slug: 'not a slug' })).status()).toBe(422);
  await submit(editor, 'main form', news);
  await expect(editor).toHaveURL(/\/admin\/news\/[a-f0-9-]+\/edit$/);
  evidence.push(['cms.news_created', new URL(editor.url()).pathname.split('/')[3]]);

  await open(editor, '/admin/events');
  await editor.locator('main a[href="/admin/events/create"]').click();
  const eventFields = {
    starts_at: new Date(now - 3_600_000).toISOString().slice(0, 16),
    ends_at: new Date(now + 86_400_000).toISOString().slice(0, 16),
    en_title: en.event, en_slug: key, en_summary: en.body, en_body: en.body,
    pl_title: pl.event, pl_slug: key, pl_summary: pl.body, pl_body: pl.body,
  };
  await denied(editor, 'main form', eventFields, player, visitor);
  expect((await responseForForm(editor, 'main form', { ...eventFields, ends_at: eventFields.starts_at })).status()).toBe(422);
  await submit(editor, 'main form', eventFields);
  await expect(editor).toHaveURL(/\/admin\/events\/[a-f0-9-]+\/edit$/);
  const event = new URL(editor.url()).pathname.split('/')[3];
  await action(editor, 'main form[action$="/status"]', { status: 'published' });
  await expect(editor.locator('main')).toContainText(lang === 'pl' ? 'Opublikowane' : 'Published');
  evidence.push(['events.event_created', event], ['events.status_changed', event]);

  await open(editor, '/admin/wiki');
  await editor.locator('main a[href="/admin/wiki/articles/create"]').click();
  const wiki = {
    en_title: en.wiki, en_slug: key, en_summary: en.body, en_markdown: en.body,
    pl_title: pl.wiki, pl_slug: key, pl_summary: pl.body, pl_markdown: pl.body,
    note: 'Create bilingual guide for the staff journey',
  };
  await denied(editor, 'main form', wiki, player, visitor);
  expect((await responseForForm(editor, 'main form', { ...wiki, en_slug: 'not a slug' })).status()).toBe(422);
  await submit(editor, 'main form', wiki);
  await expect(editor).toHaveURL(/\/admin\/wiki\/articles\/[a-f0-9-]+\/edit$/);
  const guide = new URL(editor.url()).pathname.split('/')[4];
  await submit(editor, 'main form[action$="/submit-review"]');
  await submit(editor, 'main form[action$="/publish"]');
  await expect(editor.locator('main form[action$="/unpublish"]')).toBeVisible();
  evidence.push(['wiki.created', guide], ['wiki.submit-review', guide], ['wiki.publish', guide]);

  const policies = [
    ['terms-of-service', 'terms', 'Journey terms', 'Regulamin wyprawy'],
    ['privacy-policy', 'privacy', 'Journey privacy policy', 'Polityka prywatności wyprawy'],
    ['cookie-policy', 'cookies', 'Journey cookie policy', 'Polityka plików cookie wyprawy'],
  ];
  for (const [slug, , enTitle, plTitle] of policies) {
    for (const [locale, title] of [['en', enTitle], ['pl', plTitle]]) {
      await open(editor, '/admin/pages');
      const form = editor.locator('main form[method=get]');
      await form.locator('[name=slug]').fill(slug);
      await form.locator('[name=locale]').selectOption(locale);
      await form.locator('button').click();
      await expect(editor).toHaveURL(new RegExp(`/admin/pages/${slug}/${locale}$`));
      const edit = editor.locator('main form[method=post]');
      const fields = {
        title, body: translated[locale].body, published_at: date,
        legal_version: key, legal_effective_date: date.slice(0, 10),
      };
      if (locale === 'en') {
        await denied(editor, 'main form[method=post]', fields, player, visitor, 'save');
        expect((await responseForForm(editor, 'main form[method=post]', {
          ...fields, legal_version: '',
        }, editor, 'save')).status()).toBe(400);
      }
      for (const [name, value] of Object.entries(fields)) await edit.locator(`[name=${name}]`).fill(value);
      await submit(editor, 'main form[method=post]');
      await expect(editor).toHaveURL(/\/admin\/pages$/);
      await editor.locator(`main a[href="/admin/pages/${slug}/${locale}"]`).click();
      await expect(edit.locator('[name=title]')).toHaveValue(title);
    }
    evidence.push(['cms.page.saved', slug]);
  }

  await open(visitor, `/${lang}/`);
  for (const locale of [lang, lang === 'pl' ? 'en' : 'pl']) {
    const copy = translated[locale];
    const verify = async heading => {
      await expect(visitor.locator('html')).toHaveAttribute('lang', locale);
      await expect(visitor.locator('main')).toContainText(heading);
      await expect(visitor.locator('main')).toContainText(copy.body);
    };

    for (const [section, route, title] of [
      ['news', `news/${key}`, copy.news],
      ['events', `events/${key}`, copy.event],
    ]) {
      await visitor.locator(`footer a[href="/${locale}/${section}"]`).click();
      await visitor.locator(`main a[href="/${locale}/${route}"]`).click();
      await verify(title);
    }

    const wikiIndex = `/${locale}/wiki`;
    const indexResponse = visitor.waitForResponse(response =>
      response.request().isNavigationRequest() && new URL(response.url()).pathname === wikiIndex);
    await visitor.locator(`footer a[href="${wikiIndex}"]`).click();
    expect((await indexResponse).status()).toBe(503);
    // The combined listing needs Game catalogue data. Published guides have
    // an independent public read even when the listing cannot show a link.
    await open(visitor, `/${locale}/wiki/articles/${key}`);
    await verify(copy.wiki);

    for (const [, policy, english, polish] of policies) {
      const path = `/${locale}/legal/${policy}`;
      if (policy === 'cookies') await open(visitor, path);
      else await visitor.locator(`footer a[href="${path}"]`).click();
      await verify(locale === 'pl' ? polish : english);
    }
    if (locale === lang) {
      await visitor.locator(`footer a[hreflang="${lang === 'pl' ? 'en' : 'pl'}"]`).click();
    }
  }
}
