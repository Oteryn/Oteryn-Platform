import { test, expect } from '../fixtures.mjs';
import { submit } from '../helpers.mjs';
import { messages, seedEditor } from '../public-depth-helpers.mjs';
import { assertShell } from '../shell.mjs';

test('staff publications reach the localized news archive, feed and changelog', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const root = `/${lang}`;
  const news = `${root}/news`;
  const changelog = `${root}/changelog`;
  const feed = `${news}/rss.xml`;
  expect((await page.request.get(feed)).status()).toBe(200);
  const t = messages('templates/content', lang);

  const marker = `gap-${crypto.randomUUID()}`;
  const slug = kind => `${kind}-${marker}`;
  const title = (kind, locale) => `${locale.toUpperCase()} ${kind} & ${marker}`;
  const editor = await seedEditor(browser, info);
  const edits = [];
  const create = async (kind, published, release = false) => {
    expect((await editor.page.goto(`/admin/news/create?lang=${lang}`)).status()).toBe(200);
    const editorForm = editor.page.locator('main form').first();
    await editorForm.locator('[name=kind]').selectOption(release ? 'changelog' : 'news');
    await submit(editor.page, 'main form', {
      slug: slug(kind),
      title: title(kind, 'en'),
      title_pl: title(kind, 'pl'),
      body: `Version details EN ${marker}`,
      body_pl: `Szczegóły wersji PL ${marker}`,
      published_at: published,
      translation_published_at: published,
      tags: kind === 'other-tag' ? 'other' : `updates, ${marker}`,
    });
    await expect(editor.page).toHaveURL(/\/admin\/news\/[\da-f-]+\/edit(?:\?lang=\w+)?$/);
    edits.push(new URL(editor.page.url()).pathname);
  };

  try {
    await create('archived', '2022-02-10T12:00:00Z');
    await create('other-month', '2022-03-10T12:00:00Z');
    await create('other-year', '2023-02-10T12:00:00Z');
    await create('other-tag', '2022-02-11T12:00:00Z');
    await create('future', '2099-02-10T12:00:00Z');
    await create('release', '2022-02-10T12:00:00Z', true);
    await create('draft-release', '', true);

    expect((await page.goto(news)).status()).toBe(200);
    await expect(page.locator('head link[rel=alternate][type="application/rss+xml"]')).toHaveAttribute('href', feed);
    await page.getByLabel(t.year, { exact: true }).fill('2022');
    await page.getByLabel(t.month, { exact: true }).fill('2');
    await page.getByLabel(t.tag, { exact: true }).fill(marker);
    await page.getByRole('button', { name: t.filter, exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`/news\\?.*tag=${marker}`));
    await assertShell(page);
    const archive = page.locator('main article');
    await expect(archive).toHaveCount(1);
    await expect(archive.getByRole('link', { name: title('archived', lang) })).toHaveAttribute('href', `${news}/${slug('archived')}`);
    expect((await page.request.get(`${news}?year=2022&month=13`)).status()).toBe(422);
    expect((await page.request.get(`${news}?page=100001`)).status()).toBe(422);
    expect((await page.request.get(`${news}/${slug('future')}`)).status()).toBe(404);

    const response = await page.request.get(feed);
    expect(response.headers()['content-type']).toContain('application/rss+xml');
    const rss = await page.evaluate(xml => {
      const doc = new DOMParser().parseFromString(xml, 'application/xml');
      return {
        error: doc.querySelector('parsererror')?.textContent,
        version: doc.documentElement.getAttribute('version'),
        lang: doc.querySelector('channel > language')?.textContent,
        items: [...doc.querySelectorAll('item')].map(item => Object.fromEntries(
          [...item.children].map(field => [field.tagName, field.textContent]),
        )),
      };
    }, await response.text());
    expect(rss.error).toBeUndefined();
    expect(rss.version).toBe('2.0');
    expect(rss.lang).toBe(lang);
    const item = rss.items.find(item => item.title === title('archived', lang));
    expect(item?.link).toBe(new URL(`${news}/${slug('archived')}`, page.url()).href);
    expect(Date.parse(item?.pubDate)).toBe(Date.parse('2022-02-10T12:00:00Z'));
    expect(rss.items.some(item => /\b(future|release)\b/.test(item.title) && item.title.includes(marker))).toBe(false);
    expect(rss.items.some(item => item.title === title('archived', lang === 'pl' ? 'en' : 'pl'))).toBe(false);

    await page.locator(`main a[href="${changelog}"]`).click();
    await expect(page).toHaveURL(new RegExp(`${changelog}$`));
    await page.getByLabel(t.search, { exact: true }).fill(lang === 'pl' ? `wersji PL ${marker}` : `details EN ${marker}`);
    await page.getByRole('button', { name: t.search, exact: true }).click();
    await assertShell(page);
    await expect(page.locator('main article')).toHaveCount(1);
    await page.getByRole('link', { name: title('release', lang) }).click();
    await expect(page.locator('main h1')).toHaveText(title('release', lang));
    expect((await page.request.get(`${news}/${slug('draft-release')}`)).status()).toBe(404);
  } finally {
    try {
      for (const path of edits.reverse()) {
        await editor.page.goto(path);
        await submit(editor.page, 'main form[action$="/delete"]');
        await expect(editor.page).toHaveURL(/\/admin\/news$/);
      }
    } finally {
      await editor.context.close();
    }
  }
});
