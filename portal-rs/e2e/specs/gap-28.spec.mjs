import { randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { submit } from '../helpers.mjs';
import { seededStaff } from '../staff-rbac-helpers.mjs';

const historyLabels = {
  en: { history: 'Revision history', compare: 'Compare', restored: 'Restored from revision', restore: 'Restore as draft', before: 'Before', after: 'After' },
  pl: { history: 'Historia zmian', compare: 'Porównaj', restored: 'Przywrócono z wersji', restore: 'Przywróć jako szkic', before: 'Przed', after: 'Po' },
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
    await submit(page, form, { ...fields, title: slug, body: original });
    if (pageEdit) await page.goto(pageEdit);
    else await expect(page).toHaveURL(/\/edit$/);
    const edit = new URL(page.url()).pathname;
    await submit(page, form, { body: later });
    await page.goto(edit);
    await page.getByRole('link', { name: word.history }).click();
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
    expect(await page.locator('main del em').count()).toBe(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);

    const restore = 'main ol > li:last-child form';
    await expect(page.locator(restore).getByRole('button', { name: word.restore })).toBeVisible();
    const action = await page.locator(restore).getAttribute('action');
    const oldForm = await page.locator(restore).evaluate(element => Object.fromEntries(new FormData(element)));
    await submit(page, restore);
    await expect(page.locator(`${form} [name=body]`)).toHaveValue(original);
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
    await page.goto(history);
    await expect(revisions).toHaveCount(3);
  });
}
