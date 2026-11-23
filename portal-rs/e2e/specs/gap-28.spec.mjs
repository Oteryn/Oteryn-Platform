import { randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { submit } from '../helpers.mjs';
import { query, seededStaff } from '../staff-rbac-helpers.mjs';

const historyLabels = {
  en: { history: 'Revision history', compare: 'Compare', restored: 'Restored from revision', restore: 'Restore as draft', before: 'Before', after: 'After', missingSource: 'Save the English source before restoring a translation.' },
  pl: { history: 'Historia zmian', compare: 'Porównaj', restored: 'Przywrócono z wersji', restore: 'Przywróć jako szkic', before: 'Przed', after: 'Po', missingSource: 'Zapisz wersję angielską przed przywróceniem tłumaczenia.' },
};

const entries = [
  {
    kind: 'news',
    create: slug => ['/admin/news/create', { slug }],
    form: 'main form:has([name=slug])',
  },
  {
    kind: 'page',
    create: slug => [`/admin/pages/${slug}/en`, {}],
    form: 'main form:has([name=reviewed_source_revision])',
  },
  {
    kind: 'announcement',
    create: () => ['/admin/announcements/create', {}],
    form: 'main form:has([name=lock_version])',
  },
];

for (const { kind, create, form } of entries) {
  test(`${kind} revision history, comparison and restore with i18n-parity`, async ({ page }, info) => {
    const lang = info.project.metadata.locale;
    const word = historyLabels[lang];
    const actor = await seededStaff(page, lang, 'platform_admin');
    const slug = `editorial-${randomUUID()}`;
    const [createUrl, fields] = create(slug);
    const original = 'Earlier draft <em>with literal markup</em> ' + 'a'.repeat(420);
    const later = 'Updated draft';
    expect((await page.goto(createUrl)).status()).toBe(200);
    const pageEdit = kind === 'page' ? new URL(page.url()).pathname : null;
    const editor = page.locator(form);
    const publish = kind === 'announcement' ? {} : { published_at: '2020-01-01T00:00:00Z' };
    if (kind === 'announcement') await editor.locator('[name=publication_state]').selectOption('published');
    await submit(page, form, { ...fields, ...publish, title: slug, body: original });
    if (pageEdit) await page.goto(pageEdit);
    else await expect(page).toHaveURL(/\/edit$/);
    const edit = new URL(page.url()).pathname;
    await submit(page, form, { body: later });
    if (kind === 'news') {
      expect((await page.goto('/admin/news?lang=pl')).status()).toBe(200);
      await page.getByRole('link', { name: slug, exact: true }).click();
    } else {
      expect((await page.goto(edit)).status()).toBe(200);
    }
    const opened = page.waitForResponse(response => response.request().isNavigationRequest() && new URL(response.url()).pathname.endsWith('/revisions'));
    await page.getByRole('link', { name: word.history }).click();
    const historyResponse = await opened;
    expect(historyResponse.status()).toBe(200);
    expect(historyResponse.headers()['cache-control']).toContain('no-store');
    await expect(page.locator('main#main')).toHaveCount(1);
    const history = new URL(page.url()).pathname;
    const revisions = page.locator('main ol > li');
    await expect(revisions).toHaveCount(2);
    await expect(revisions.first()).toContainText(actor);
    await expect(revisions.first().locator('time')).toHaveAttribute('datetime', /T.*Z$/);
    await expect(page.locator('html')).toHaveAttribute('lang', lang);

    await page.getByRole('button', { name: word.compare, exact: true }).click();
    await expect(page.getByRole('heading', { name: word.before, exact: true })).toBeVisible();
    await expect(page.getByRole('heading', { name: word.after, exact: true })).toBeVisible();
    await expect(page.locator('main del')).toHaveText(original);
    await expect(page.locator('main ins')).toHaveText(later);
    await expect(page.locator('main del textarea')).toHaveValue(original);
    await expect(page.locator('main ins textarea')).toHaveValue(later);
    expect(await page.locator('main del em').count()).toBe(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);

    const restore = 'main ol > li:last-child form';
    await expect(page.locator(restore).getByRole('button', { name: word.restore })).toBeVisible();
    const action = await page.locator(restore).getAttribute('action');
    const oldForm = await page.locator(restore).evaluate(element => Object.fromEntries(new FormData(element)));
    await submit(page, restore);
    await expect(editor.locator('[name=body]')).toHaveValue(original);
    if (kind === 'announcement') {
      await expect(editor.locator('[name=publication_state]')).toHaveValue('draft');
    } else {
      await expect(editor.locator('[name=published_at]')).toHaveValue('');
    }
    await page.getByRole('link', { name: word.history }).click();
    await expect(revisions).toHaveCount(3);
    await expect(revisions.first()).toContainText(word.restored);
    await expect(revisions.first()).toContainText(actor);

    const stale = await page.request.post(action, {
      form: oldForm,
      headers: { origin: new URL(page.url()).origin },
      maxRedirects: 0,
    });
    expect(stale.status()).toBe(409);
    expect((await page.request.get(`${history}?from=1`)).status()).toBe(400);
    const invalid = await page.request.post(action, {
      form: { ...oldForm, expected_revision: kind === 'announcement' ? '0' : '' },
      headers: { origin: new URL(page.url()).origin },
      maxRedirects: 0,
    });
    expect(invalid.status()).toBe(400);
    await page.goto(history);
    await expect(revisions).toHaveCount(3);

    if (kind === 'news') {
      const opposite = lang === 'en' ? 'pl' : 'en';
      const origin = new URL(page.url()).origin;
      const setLanguage = value => page.context().addCookies([{ name: 'portal_language', value, url: origin }]);
      await setLanguage(opposite);
      await page.reload();
      await expect(page.getByRole('heading', { name: historyLabels[opposite].history })).toBeVisible();
      await expect(page.locator('html')).toHaveAttribute('lang', opposite);
      await setLanguage(lang);

      const id = edit.split('/')[3];
      await query(
        `INSERT INTO editorial_revisions(kind,object_key,locale,actor,snapshot)
         SELECT kind,object_key,locale,actor,snapshot
         FROM (SELECT * FROM editorial_revisions WHERE kind='news' AND object_key=$1 ORDER BY id DESC LIMIT 1) latest
         CROSS JOIN generate_series(1,9)`,
        [id],
      );
      await page.reload();
      await expect(revisions).toHaveCount(10);
      const seen = await revisions.locator('h3').allTextContents();
      await page.locator('main a[rel=next]').click();
      await expect(revisions).toHaveCount(2);
      const older = await revisions.locator('h3').allTextContents();
      expect(new Set([...seen, ...older]).size).toBe(12);
      expect(older.every(title => Number(title.match(/^#(\d+)/)[1]) < Number(seen.at(-1).match(/^#(\d+)/)[1]))).toBe(true);
      await page.goto(edit);
      await submit(page, 'main form[action$="/delete"]');
      await expect(page).toHaveURL(/\/admin\/news$/);
      expect((await page.goto(`${history}?before=1`)).status()).toBe(200);
      await expect(page.locator('main a[href="/admin/news"]').first()).toBeVisible();
      await expect(page.locator('main form[method=post]')).toHaveCount(0);
    }
    if (kind === 'page') {
      const translation = `${edit.slice(0, -2)}pl`;
      expect((await page.goto(translation)).status()).toBe(200);
      await submit(page, form, { title: 'Polska wersja', body: 'Tekst tłumaczenia' });
      await expect(page).toHaveURL(/\/admin\/pages$/);
      await page.goto(edit);
      await page.locator(`${form} button[value=delete]`).click();
      await expect(page).toHaveURL(/\/admin\/pages$/);
      await page.goto(edit);
      await page.getByRole('link', { name: word.history }).click();
      await expect(revisions).toHaveCount(3);
      await expect(page.locator('main form[method=post]')).toHaveCount(0);
      expect((await page.goto(`${translation}/revisions`)).status()).toBe(200);
      await expect(page.getByText(word.missingSource, { exact: true })).toBeVisible();
      await expect(page.locator('main form[method=post]')).toHaveCount(0);
    }
  });
}
