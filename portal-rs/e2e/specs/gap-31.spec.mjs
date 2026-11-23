import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { submit } from '../helpers.mjs';
import { expectLoginRedirect, query, seededStaff } from '../staff-rbac-helpers.mjs';

test('staff sees saved editorial previews in both languages', async ({ page, playwright }, info) => {
  test.setTimeout(150_000);
  const lang = info.project.metadata.locale;
  const actor = await seededStaff(page, lang, 'platform_admin');
  const slug = `preview-${randomUUID()}`;
  const title = { en: `English ${slug}`, pl: `Polski ${slug}` };
  const body = { en: 'English editorial body', pl: 'Polska treść redakcyjna' };
  const unsafe = '<span onclick="alert(1)">Editorial markup</span>';
  const notices = {
    en: 'Staff preview of the saved revision.',
    pl: 'Podgląd zapisanej wersji dla redakcji.',
  };
  const start = new Date(Date.now() - 60_000).toISOString();
  const end = new Date(Date.now() + 3_600_000).toISOString();
  const anonymous = await playwright.request.newContext({
    baseURL: new URL(page.url()).origin,
    ignoreHTTPSErrors: true,
  });

  async function openEditor(kind, suffix) {
    expect((await page.goto('/admin')).status()).toBe(200);
    await page.locator(`a[href="/admin/${kind}"]`).first().click();
    if (kind === 'pages') {
      await page.locator('main input[name=slug]').fill(slug);
      await page.locator('main select[name=locale]').selectOption(suffix);
      await page.locator('main form button').click();
    } else {
      await page.locator(`main a[href^="/admin/${kind}/create"]`).click();
    }
  }

  const previewPaths = [];
  async function preview(locale, selector, expectedTitle = title[locale]) {
    const link = page.locator(`main a[hreflang="${locale}"][href*="/preview/"]`);
    await expect(link).toBeVisible();
    const path = await link.getAttribute('href');
    const result = await Promise.all([page.waitForNavigation(), link.click()]).then(([result]) => result);
    expect(result.status()).toBe(200);
    expect(result.headers()['cache-control']).toContain('no-store');
    expect(result.headers()['content-language']).toBe(locale);
    expect(result.headers()['x-robots-tag']).toBe('noindex, nofollow');
    await expect(page.locator('html')).toHaveAttribute('lang', locale);
    await expect(page.locator('a.skip[href="#main"]')).toHaveCount(1);
    await expect(page.locator('footer')).toBeVisible();
    await expect(page.locator('main [role="note"]')).toHaveText(notices[locale]);
    await expect(page.locator('main').getByRole('heading', { name: expectedTitle, exact: true }).first()).toBeVisible();
    await expect(page.locator(selector)).toContainText(body[locale]);
    await expect(page.locator('main [onclick]')).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    const refused = await anonymous.get(path, { maxRedirects: 0 });
    expect(refused.status()).toBe(303);
    expect(refused.headers().location).toBe('/login');
    const [, base, revision] = path.match(/^(.*\/preview\/)\w+\/(\d+)$/);
    expect((await page.request.get(`${base}xx/${revision}`)).status()).toBe(404);
    expect((await page.request.get(`${base}${locale}/99999999999999`)).status()).toBe(404);
    expect((await page.request.get(`${base}${locale}/invalid`)).status()).toBe(400);
    previewPaths.push(path);
  }

  try {
    await openEditor('news');
    await submit(page, 'main form', {
      slug, title: title.en, body: `${body.en} ${unsafe}`,
      title_pl: title.pl, body_pl: `${body.pl} ${unsafe}`,
    });
    await expect(page).toHaveURL(/\/admin\/news\/[\da-f-]+\/edit$/);
    const newsEdit = new URL(page.url()).pathname;
    for (const locale of ['en', 'pl']) {
      await page.goto(newsEdit);
      await preview(locale, 'main article');
      expect((await anonymous.get(`/${locale}/news/${slug}`)).status()).toBe(404);
    }
    await page.goto(newsEdit);
    await submit(page, 'main form:not([action$="/delete"])', { title: `Updated ${title.en}` });
    expect((await page.request.get(previewPaths[0])).status()).toBe(404);

    for (const locale of ['en', 'pl']) {
      await openEditor('pages', locale);
      const edit = new URL(page.url()).pathname;
      await submit(page, 'main form[method=post]', {
        title: title[locale], body: `**${body[locale]}**\n\n${unsafe}`,
      });
      await expect(page).toHaveURL(/\/admin\/pages$/);
      await page.goto(edit);
      await preview(locale, 'main article strong');
      expect((await anonymous.get(`/${locale}/pages/${slug}`)).status()).toBe(404);
    }

    await page.goto(`/admin/pages/${slug}/pl`);
    for (const locale of ['en', 'pl']) {
      await expect(page.locator(`main a[hreflang="${locale}"][href*="/preview/"]`))
        .toHaveAttribute('href', previewPaths[locale === 'en' ? 2 : 3]);
    }

    await openEditor('events');
    await submit(page, 'main form', {
      starts_at: start.slice(0, 16), ends_at: end.slice(0, 16), reason: 'Review draft appearance',
      en_title: title.en, en_slug: slug, en_summary: 'English summary', en_body: `${body.en} ${unsafe}`,
      pl_title: title.pl, pl_slug: slug, pl_summary: 'Polskie podsumowanie', pl_body: `${body.pl} ${unsafe}`,
    });
    await expect(page).toHaveURL(/\/admin\/events\/[\da-f-]+\/edit$/);
    const eventEdit = new URL(page.url()).pathname;
    for (const locale of ['en', 'pl']) {
      await page.goto(eventEdit);
      await preview(locale, 'main article');
      await expect(page.locator(`main a[href="/${locale}/events/${slug}"]`)).toHaveCount(0);
      expect((await anonymous.get(`/${locale}/events/${slug}`)).status()).toBe(404);
    }

    await openEditor('announcements');
    await page.locator('select[name=severity]').selectOption('warning');
    await submit(page, 'main form', {
      starts_at: start, ends_at: end, title: title.en, body: `${body.en} ${unsafe}`,
      title_pl: title.pl, body_pl: `${body.pl} ${unsafe}`, action_url: '/worlds',
      action_label: 'Worlds',
    });
    await expect(page).toHaveURL(/\/admin\/announcements\/[\da-f-]+\/edit$/);
    const announcementEdit = new URL(page.url()).pathname;
    const draftEnglish = page.locator('main a[hreflang="en"][href*="/preview/"]');
    await expect(draftEnglish).toBeVisible();
    await expect(page.locator('main a[hreflang="pl"][href*="/preview/"]')).toHaveCount(0);
    const draftPath = await draftEnglish.getAttribute('href');
    const draftPolish = draftPath.replace('/preview/en/', '/preview/pl/');
    expect((await page.request.get(draftPolish)).status()).toBe(404);
    await submit(page, 'main form[method=post]', { action_label_pl: 'Światy' });
    await expect(page).toHaveURL(announcementEdit);
    expect((await page.request.get(draftPath)).status()).toBe(404);

    for (const locale of ['en', 'pl']) {
      await page.goto(announcementEdit);
      await preview(locale, 'main aside article', `${locale === 'pl' ? 'Ostrzeżenie' : 'Warning'}: ${title[locale]}`);
      await expect(page.locator('main aside article a')).toHaveAttribute('href', '/worlds');
      const ticker = await anonymous.get(`/${locale}/announcements/ticker`);
      expect(ticker.status()).toBe(200);
      expect(await ticker.text()).not.toContain(slug);
    }

    const currentNews = await page.goto(newsEdit);
    expect(currentNews.status()).toBe(200);
    const updatedLink = page.locator('main a[hreflang="en"][href*="/preview/"]');
    const updatedPath = await updatedLink.getAttribute('href');
    const currentPaths = [updatedPath, ...previewPaths.slice(2)];
    await query("DELETE FROM account_roles WHERE account_id=$1", [actor]);
    await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='security_admin'", [actor]);
    expect((await page.goto('/admin')).status()).toBe(200);
    for (const kind of ['news', 'pages', 'events', 'announcements']) {
      await expect(page.locator(`main a[href="/admin/${kind}"]`)).toHaveCount(0);
    }
    for (const path of currentPaths) {
      expect((await page.request.get(path, { maxRedirects: 0 })).status()).toBe(403);
    }
    await query("UPDATE web_sessions SET idle_expires_at=now()-interval '1 second' WHERE account_id=$1", [actor]);
    for (const path of currentPaths) await expectLoginRedirect(page, path);
  } finally {
    await anonymous.dispose();
  }
});


test('i18n-parity for editorial preview and staff navigation', () => {
  const keys = (section, locale) => [...readFileSync(
    new URL(`../../templates/${section}/${locale}.ftl`, import.meta.url), 'utf8',
  ).matchAll(/^([a-z][\w-]+) = \S+/gm)].map(([, key]) => key).sort();
  for (const [section, required] of [
    ['pages', ['preview-link', 'preview-notice']],
    ['admin', ['cms-news', 'cms-pages', 'cms-events', 'cms-announcements']],
  ]) {
    const english = keys(section, 'en');
    expect(english).toEqual(expect.arrayContaining(required));
    expect(keys(section, 'pl')).toEqual(english);
  }
});
