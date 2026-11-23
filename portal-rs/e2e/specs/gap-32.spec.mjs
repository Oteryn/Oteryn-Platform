import {randomUUID} from 'node:crypto';
import {test, expect} from '../fixtures.mjs';
import {submit} from '../helpers.mjs';
import {assertShell} from '../shell.mjs';
import {query, seededStaff} from '../staff-rbac-helpers.mjs';

const form = 'main form[data-discovery]';

async function configure(page, key, position) {
  await page.locator(`${form} select[name=placement]`).selectOption('community');
  await submit(page, form, {
    menu_key: key,
    menu_position: String(position),
    menu_label_en: 'Editorial calendar',
    menu_label_pl: 'Kalendarz redakcyjny',
    title_en: 'Calendar & updates',
    title_pl: 'Kalendarz i aktualności',
    description_en: 'Updates <and> dates',
    description_pl: 'Aktualności i daty',
  });
}

test('staff place a published editorial document in both menus and the sitemap', async ({page, browser}, info) => {
  const locale = info.project.metadata.locale;
  const position = ['pl-1440', 'pl-390', 'en-1440', 'en-390'].indexOf(info.project.name) + 1;
  const key = `calendar-${randomUUID()}`;
  const paths = {en: `/en/events/${key}-en`, pl: `/pl/events/${key}-pl`};
  const role = key.replaceAll('-', '_');
  await query('INSERT INTO roles(key) VALUES ($1)', [role]);
  await query(
    "INSERT INTO role_permissions(role_id, permission) SELECT id, unnest(ARRAY['admin.access', 'portal.access', 'events.manage', 'events.publish']) FROM roles WHERE key=$1",
    [role],
  );
  await seededStaff(page, locale, role);
  await page.goto('/admin/events/create');
  await submit(page, 'main form', {
    starts_at: new Date(Date.now() - 3_600_000).toISOString().slice(0, 16),
    ends_at: new Date(Date.now() + 86_400_000).toISOString().slice(0, 16),
    reason: 'Create calendar event',
    en_slug: `${key}-en`, en_title: 'Calendar event', en_summary: 'English summary', en_body: 'English body.',
    pl_slug: `${key}-pl`, pl_title: 'Wydarzenie w kalendarzu', pl_summary: 'Polskie podsumowanie', pl_body: 'Polska treść.',
  });
  await expect(page).toHaveURL(/\/admin\/events\/[a-f0-9-]+\/edit$/);
  const edit = new URL(page.url()).pathname;
  const settings = page.locator('main a[href^="/admin/discovery/events/"]');
  await expect(settings).toBeVisible();
  await settings.click();
  const discovery = new URL(page.url()).pathname;
  await assertShell(page);
  const localizedHeading = {pl: 'Menu i wyszukiwarki', en: 'Menu and search engines'};
  await expect(page.locator('main h1')).toHaveText(localizedHeading[locale]);

  const context = await browser.newContext({
    baseURL: new URL(page.url()).origin, ignoreHTTPSErrors: true,
    viewport: info.project.use.viewport, locale,
  });
  try {
    const visitor = await context.newPage();
    const sitemap = async () => {
      const result = await visitor.request.get('/sitemap.xml');
      expect(result.ok()).toBe(true);
      return result.text();
    };
    for (const path of Object.values(paths)) {
      expect((await visitor.request.get(path)).status()).toBe(404);
      expect(await sitemap()).not.toContain(path);
    }
    for (const invalid of ['0', '50001', 'abc']) {
      expect((await visitor.request.get(`/sitemap.xml?page=${invalid}`)).status()).toBe(400);
    }
    expect((await visitor.request.get('/sitemap.xml?page=50000')).status()).toBe(404);
    const refused = page.waitForResponse(r => new URL(r.url()).pathname === discovery && r.request().method() === 'POST' && r.status() !== 403);
    await configure(page, key, position);
    expect((await refused).status()).toBe(409);
    await expect(page.getByRole('alert')).toHaveText(locale === 'pl'
      ? 'Opublikuj obie wersje językowe przed dodaniem dokumentu do menu.'
      : 'Publish both translations before adding the document to a menu.');

    await page.goto(edit);
    await page.locator('main form[action$="/status"] select[name=status]').selectOption('published');
    await submit(page, 'main form[action$="/status"]', {reason: 'Publish calendar event'});
    await page.goto(discovery);
    await configure(page, key, position);
    await expect(page).toHaveURL(/[?&]saved=1$/);

    const origin = new URL(page.url()).origin;
    const index = await sitemap();
    for (const lang of [locale, locale === 'pl' ? 'en' : 'pl']) {
      const other = lang === 'pl' ? 'en' : 'pl';
      const label = lang === 'pl' ? 'Kalendarz redakcyjny' : 'Editorial calendar';
      await visitor.goto(`/${lang}/`);
      if (info.project.use.viewport.width === 390) {
        await visitor.locator('#portal-menu-toggle').click();
      }
      await visitor.locator('#portal-community-button').click();
      const link = visitor.locator(`#portal-community-panel a[href="${paths[lang]}"]`);
      await expect(link).toHaveText(label);
      await link.click();
      await expect(visitor).toHaveURL(paths[lang]);
      await expect(visitor).toHaveTitle(lang === 'pl' ? 'Kalendarz i aktualności' : 'Calendar & updates');
      await expect(visitor.locator('meta[name=description]')).toHaveAttribute('content', lang === 'pl' ? 'Aktualności i daty' : 'Updates <and> dates');
      await expect(visitor.locator('link[rel=canonical]')).toHaveAttribute('href', origin + paths[lang]);
      await expect(visitor.locator(`head link[hreflang=${other}]`)).toHaveAttribute('href', origin + paths[other]);
      expect(index).toContain(`<loc>${origin}${paths[lang]}</loc>`);
      expect(index).toContain(`hreflang="${other}" href="${origin}${paths[other]}"`);
      await expect(visitor.locator(`#portal-community-panel a[href="${paths[lang]}"]`)).toHaveAttribute('aria-current', 'page');
      expect(await visitor.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    }

    await page.locator(`${form} select[name=indexing]`).selectOption('noindex');
    await submit(page, form);
    await expect(page).toHaveURL(/[?&]saved=1$/);
    await visitor.reload();
    await expect(visitor.locator('meta[name=robots]')).toHaveAttribute('content', 'noindex');
    for (const path of Object.values(paths)) {
      expect(await sitemap()).not.toContain(path);
      const lang = path.split('/')[1];
      const alias = path.replace(`/${lang}/`, `/%${lang.charCodeAt(0).toString(16)}${lang[1]}/`)
        .replace('/events/calendar-', '/events/%63alendar-');
      const aliasResponse = await visitor.request.get(alias);
      expect(aliasResponse.status()).toBe(200);
      expect(aliasResponse.headers()['x-robots-tag']).toBe('noindex');
      expect(aliasResponse.headers()['content-language']).toBe(lang);
      const canonical = await visitor.goto(alias);
      expect(canonical.status()).toBe(200);
      await expect(visitor.locator('link[rel=canonical]')).toHaveAttribute('href', origin + path);
    }

    await page.goto(edit);
    await page.locator('main form[action$="/status"] select[name=status]').selectOption('archived');
    await submit(page, 'main form[action$="/status"]', {reason: 'Archive calendar event'});
    await visitor.goto(`/${locale}/`);
    await expect(visitor.locator(`#portal-community-panel a[href="${paths[locale]}"]`)).toHaveCount(0);
    for (const path of Object.values(paths)) expect((await visitor.request.get(path)).status()).toBe(404);

    await page.goto(discovery);
    await page.locator(`${form} select[name=placement]`).selectOption('');
    await submit(page, form);
    await expect(page).toHaveURL(/\?lang=[a-z]+&saved=1$/);

    await query(
      "DELETE FROM role_permissions USING roles WHERE role_id=roles.id AND roles.key=$1 AND permission='events.publish'",
      [role],
    );
    expect((await page.goto(edit)).status()).toBe(200);
    await expect(page.locator('main a[href^="/admin/discovery/events/"]')).toHaveCount(0);
    expect((await page.request.get(discovery)).status()).toBe(403);
  } finally {
    await context.close();
  }
});

test('i18n-parity for discovery controls and feedback', async ({page}, info) => {
  const locale = info.project.metadata.locale;
  await seededStaff(page, locale, 'platform_admin');
  await page.goto('/admin/news/create');
  const key = `metadata-${randomUUID()}`;
  await submit(page, 'main form', {slug: key, title: 'Metadata draft', body: 'English draft.'});
  await page.locator('main a[href^="/admin/discovery/news/"]').click();
  const settings = new URL(page.url()).pathname;
  for (const lang of ['pl', 'en']) {
    await page.goto(`${settings}?lang=${lang}`);
    await expect(page.locator('main h1')).toHaveText(lang === 'pl' ? 'Menu i wyszukiwarki' : 'Menu and search engines');
    const other = lang === 'pl' ? 'en' : 'pl';
    await expect(page.locator(`footer a[hreflang=${other}]`)).toHaveAttribute('href', `${settings}?lang=${other}`);
    await expect(page.locator(`${form} label`)).toHaveCount(10);
    for (const label of await page.locator(`${form} label`).all()) {
      await expect(label).not.toHaveText('');
      const field = await label.getAttribute('for');
      await expect(page.locator(`${form} #${field}`)).toBeVisible();
    }
    const {action, fields} = await page.locator(form).evaluate(element => ({
      action: element.action, fields: Object.fromEntries(new FormData(element)),
    }));
    const invalid = await page.request.post(action, {
      form: {...fields, menu_position: '9'}, headers: {origin: new URL(page.url()).origin},
    });
    expect(invalid.status()).toBe(422);
    expect(await invalid.text()).toContain(lang === 'pl' ? 'Sprawdź' : 'Check');
    await submit(page, form);
    await expect(page.getByRole('status')).toHaveText(lang === 'pl' ? 'Zapisano ustawienia.' : 'Settings saved.');
    await assertShell(page);
  }
});
