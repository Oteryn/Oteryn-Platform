import { randomUUID } from 'node:crypto';
import { expect, test } from '../fixtures.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { expectRedirect, parallelSubmit, reloadGet, signIn, snapshot, submitForm } from './forms.mjs';

const index = '/admin/pages';
const copy = {
  en: {
    index: 'Pages',
    conflict: 'This change conflicts with the current page. Reload the editor and review your changes.',
  },
  pl: {
    index: 'Strony',
    conflict: 'Ta zmiana koliduje z aktualną wersją strony. Odśwież edytor i sprawdź swoje zmiany.',
  },
};

function save(page) {
  return page.locator('main form button[value=save]');
}

async function fill(page, title, body) {
  await page.locator('main [name=title]').fill(title);
  await page.locator('main [name=body]').fill(body);
  await page.locator('main [name=published_at]').fill('2020-01-01T00:00:00Z');
  return snapshot(save(page));
}

async function pages(slug, lang) {
  return query(
    'SELECT e.title, e.body, e.revision::text FROM editorial_pages e JOIN pages p ON p.id=e.page_id WHERE p.slug=$1 AND e.locale=$2',
    [slug, lang],
  );
}

async function audit(slug, lang) {
  return query(
    "SELECT actor, action, after->>'title' AS title FROM audit_log WHERE target=$1 AND after->>'locale'=$2 ORDER BY id",
    [slug, lang],
  );
}

async function openEditor(page, editor, lang) {
  expect((await page.goto(editor)).status()).toBe(200);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(copy[lang].index);
}

test('parallel CMS saves and a stale second tab preserve the accepted revision and its audit', async ({ page, context }, info) => {
  const lang = info.project.metadata.locale;
  const actor = await signIn(page, lang, 'content_editor');
  const slug = `double-submit-${randomUUID()}`;
  const editor = `${index}/${slug}/${lang}`;
  const winner = lang === 'pl' ? 'Strona po zapisie' : 'Page after save';
  const body = lang === 'pl' ? 'Treść pierwszej zmiany.' : 'Content of the first edit.';
  const second = await context.newPage();

  // A translation requires an existing English source revision.
  if (lang === 'pl') {
    await openEditor(page, `${index}/${slug}/en`, 'en');
    const source = await fill(page, 'English source', 'Source for the Polish translation.');
    expectRedirect(await submitForm(page, save(page), source.url), 303, index);
    await expect(page).toHaveURL(index);
  }
  await openEditor(page, editor, lang);
  await openEditor(second, editor, lang);
  const original = await fill(page, winner, body);
  const stale = await fill(second, 'A different edit', 'Text from the older editor.');
  expect(await pages(slug, lang)).toEqual([]);
  expect(await audit(slug, lang)).toEqual([]);

  const [accepted, refused] = (await parallelSubmit(page, save(page), original))
    .sort((a, b) => a.status() - b.status());
  expectRedirect(accepted, 303, index);
  expect(refused.status()).toBe(409);
  expect(await refused.text()).toBe(copy[lang].conflict);
  const conflict = await submitForm(second, save(second), stale.url);
  expect(conflict.status()).toBe(409);
  await expect(second.locator('body')).toHaveText(copy[lang].conflict);
  expect(await pages(slug, lang)).toEqual([{ title: winner, body, revision: expect.any(String) }]);
  expect(await audit(slug, lang)).toEqual([{ actor: actor.id, action: 'cms.page.saved', title: winner }]);

  // The API request can win the race. Follow the native form's result before
  // reloading the index, then confirm the stored content on the public page.
  await page.goto(index);
  await reloadGet(page);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(copy[lang].index);
  await expect(page.locator(`a[href="${editor}"]`)).toBeVisible();
  expect((await page.goto(`/${lang}/pages/${slug}`)).status()).toBe(200);
  await expect(page.locator('main')).toContainText(body);
  expect(await audit(slug, lang)).toHaveLength(1);

  await openEditor(page, editor, lang);
  await openEditor(second, editor, lang);
  for (const tab of [page, second]) {
    await expect(tab.locator('main [name=title]')).toHaveValue(winner);
    await expect(tab.locator('main [name=body]')).toHaveValue(body);
  }
  const updatedBody = lang === 'pl' ? 'Zapisana zmiana istniejącej strony.' : 'Saved change to the existing page.';
  const existing = await fill(page, winner, updatedBody);
  const competing = await fill(second, 'Other existing edit', 'Unsaved text from the competing tab.');
  expectRedirect(await submitForm(page, save(page), existing.url), 303, index);
  await expect(page).toHaveURL(index);
  await reloadGet(page);
  const competingResponse = await submitForm(second, save(second), competing.url);
  expect(competingResponse.status()).toBe(409);
  await expect(second.locator('body')).toHaveText(copy[lang].conflict);
  expect(await pages(slug, lang)).toEqual([{ title: winner, body: updatedBody, revision: expect.any(String) }]);
  expect(await audit(slug, lang)).toEqual([
    { actor: actor.id, action: 'cms.page.saved', title: winner },
    { actor: actor.id, action: 'cms.page.saved', title: winner },
  ]);
  expect((await page.goto(`/${lang}/pages/${slug}`)).status()).toBe(200);
  await expect(page.locator('main')).toContainText(updatedBody);
});
