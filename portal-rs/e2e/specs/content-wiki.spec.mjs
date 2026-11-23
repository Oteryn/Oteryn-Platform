import { waitForWikiBudget } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import {
 test, expect, editPath, formFor, key, labels, link, locales, localized as t,
 mainForm, refuse, send, submitForm, title, visit,
} from '../content-helpers.mjs';

test('wiki draft, preview, review, search, publish, revision restore and archive', async ({content}, {project}) => {
 const { anonymous, editor } = content;
 const page = editor.page;
 const lang = project.metadata.locale;
 const list = '/admin/wiki';
 const root = `${list}/articles`;
 const slug = key('guide');
 const values = {
  en_title: `Guide ${slug}`, en_slug: slug, en_summary: 'English summary.',
  en_markdown: '## Original guide',
  pl_title: `Poradnik ${slug}`, pl_slug: slug, pl_summary: 'Polskie streszczenie.',
  pl_markdown: '## Pierwotny poradnik', note: 'Initial revision',
 };
 await refuse(content, list);
 await refuse(content, root);
 await refuse(content, `${root}/create`);
 await refuse(content, root, 'POST', values);
 await visit(page, list);
 await title(page, 'Wiki', lang);
 await link(page, `${root}/create`, t(lang, 'Nowy poradnik', 'New guide'));
 for (const locale of locales) {
  await labels(mainForm(page).getByRole('group', {name: locale, exact: true}), lang, [
   [`${locale}_title`, 'Tytuł', 'Title'], [`${locale}_slug`, 'Adres', 'Slug'],
   [`${locale}_summary`, 'Streszczenie', 'Summary'], [`${locale}_markdown`, 'Markdown', 'Markdown'],
  ]);
 }
 await labels(mainForm(page), lang, [['note', 'Opis zmiany', 'Change note']]);
 await expect(mainForm(page).getByRole('button')).toHaveText(t(lang, 'Zapisz szkic', 'Save draft'));
 await send(mainForm(page), { ...values, en_slug: 'bad slug' }, 422);
 await submitForm(mainForm(page), values);
 await expect(page).toHaveURL(/\/admin\/wiki\/articles\/[a-f0-9-]+\/edit$/);
 const edit = editPath(page);
 const row = edit.slice(0, -5);
 const transition = action => formFor(page, `${row}/${action}`);
 await refuse(content, edit);
 await refuse(content, row, 'POST', values);
 await visit(page, root);
 await link(page, edit);
 const id = row.split('/').at(-1);
 // Give history a full first page while keeping the original revision created
 // above available to restore from the continuation page.
 await query(`
  INSERT INTO wiki_revisions(id,guide_id,version,locale,title,slug,summary,markdown,editor,change_note)
  SELECT uuidv7(),guide_id,n,locale,title,slug,summary,'Older draft '||n,editor,'Historical fixture'
  FROM wiki_revisions CROSS JOIN generate_series(2,11) n
  WHERE guide_id=$1 AND version=1`, [id]);
 await query('UPDATE wiki_guides SET version=11,revision_version=11 WHERE id=$1', [id]);
 await page.reload();
 await send(mainForm(page), { en_slug: '' }, 422);
 await send(mainForm(page), { entity_key: 'x'.repeat(257) }, 422);
 await labels(mainForm(page), lang, [['entity_key', 'Powiązanie z encją', 'Entity binding']]);
 await submitForm(mainForm(page), {
  en_markdown: '## Edited guide', pl_markdown: '## Zmieniony poradnik', note: 'Draft edit', entity_key: 'item:100',
 });
 for (const locale of locales) {
  await waitForWikiBudget(`/${locale}/wiki`);
  await visit(anonymous.page, `/${locale}/wiki/articles/${slug}`, 404);
  await refuse(content, `${row}/preview/${locale}`);
  await link(page, `${row}/preview/${locale}`, `${t(lang, 'Podgląd', 'Preview')} (${locale.toUpperCase()})`);
  await title(page, values[`${locale}_title`], locale);
  await expect(page.getByRole('heading', { name: t(locale, 'Zmieniony poradnik', 'Edited guide') })).toBeVisible();
  await page.goBack();
 }
 await visit(page, `${row}/preview/de`, 404);
 await visit(page, edit);
 for (const action of ['submit-review', 'return-draft', 'publish', 'unpublish', 'archive']) {
  await refuse(content, `${row}/${action}`, 'POST', { lock_version: '1' });
 }
 await send(transition('submit-review'), { lock_version: 'bad' }, 422);
 await send(transition('submit-review'), {}, 404, 'POST', `${row}/unknown`);
 await send(transition('submit-review'), {}, 409, 'POST', `${row}/publish`);
 await submitForm(transition('submit-review'));
 await expect(page.locator('main > p').first()).toContainText(t(lang, 'W przeglądzie', 'In review'));
 await send(transition('return-draft'), { lock_version: 'bad' }, 422);
 await submitForm(transition('return-draft'));
 await expect(page.locator('main > p').first()).toContainText(t(lang, 'Szkic', 'Draft'));
 await submitForm(transition('submit-review'));
 await send(transition('publish'), { lock_version: 'bad' }, 422);
 await submitForm(transition('publish'));
 await expect(page.locator('main > p').first()).toContainText(t(lang, 'Opublikowany', 'Published'));
 const catalogEnabled = process.env.PORTAL_WIKI_ENTITIES_ENABLED === 'true';
 const unavailable = async (target, locale) => {
  const response = await visit(anonymous.page, target, 503);
  expect(response.headers()['cache-control'].split(/,\s*/)).toContain('no-store');
  await expect(anonymous.page.locator('header#portal-header')).toBeVisible();
  await title(anonymous.page, t(locale,
   'Dane świata są obecnie niedostępne.', 'World data is currently unavailable.'), locale);
  await expect(anonymous.page.locator('main')).not.toContainText(slug);
 };
 for (const locale of locales) {
  const home = `/${locale}/wiki`;
  const article = `${home}/articles/${slug}`;
  await waitForWikiBudget(home, 5);
  if (catalogEnabled) {
   // The prepared external catalogue deliberately has no accepted Game data.
   await unavailable(home, locale);
   await unavailable(`${home}/search?q=${slug}`, locale);
   await visit(anonymous.page, article);
  } else {
   await visit(anonymous.page, home);
   const search = formFor(anonymous.page, '/wiki/search');
   await search.getByLabel(t(locale, 'Szukane słowa', 'Search terms')).fill(slug);
   await search.getByRole('button', { name: t(locale, 'Szukaj poradników', 'Search guides') }).click();
   await expect(anonymous.page).toHaveURL(/\/wiki\/search\?q=guide-/);
   await link(anonymous.page, article, values[`${locale}_title`]);
  }
  await title(anonymous.page, values[`${locale}_title`], locale);
  await expect(anonymous.page.locator('main')).toContainText(values[`${locale}_summary`]);
  await expect(anonymous.page.locator('main')).toContainText(t(locale, 'Zmieniony poradnik', 'Edited guide'));
  await anonymous.page.locator(`a[hreflang="${locale}"][href="${home}"]`).click();
  await expect(anonymous.page).toHaveURL(new RegExp(`${home}$`));
  await title(anonymous.page, catalogEnabled
   ? t(locale, 'Dane świata są obecnie niedostępne.', 'World data is currently unavailable.')
   : 'Wiki', locale);
 }
 await waitForWikiBudget('/wiki', 4);
 if (catalogEnabled) {
  await unavailable('/wiki', lang);
  await unavailable(`/wiki/search?q=${slug}`, lang);
 } else {
  await visit(anonymous.page, '/wiki');
  await visit(anonymous.page, `/wiki/search?q=${slug}`);
  await link(anonymous.page, `/${lang}/wiki/articles/${slug}`, values[`${lang}_title`]);
 }
 await visit(anonymous.page, '/wiki/search?q=x', 400);
 await visit(anonymous.page, `/wiki/articles/${slug}`);
 await title(anonymous.page, values[`${lang}_title`], lang);
 await send(transition('unpublish'), { lock_version: 'bad' }, 422);
 await submitForm(transition('unpublish'));
 for (const locale of locales) {
  await waitForWikiBudget(`/${locale}/wiki`);
  await visit(anonymous.page, `/${locale}/wiki/articles/${slug}`, 404);
  await expect(anonymous.page.locator('main')).toContainText(t(locale, 'Nie znaleziono artykułu.', 'Article not found.'));
 }
 await refuse(content, `${row}/revisions`);
 await link(page, `${row}/revisions`, t(lang, 'Historia wersji', 'Revision history'));
 await title(page, t(lang, 'Historia wersji', 'Revision history'), lang);
 const older = page.getByRole('link', {name: t(lang, 'Starsze wersje', 'Older revisions')});
 await older.click();
 await expect(page).toHaveURL(/\/revisions\?before=\d+$/);
 const original = page.locator('main article').filter({ hasText: 'Original guide' });
 await expect(original).toContainText('Initial revision');
 const restore = original.locator('form');
 await refuse(content, await restore.getAttribute('action'), 'POST', { lock_version: '1' });
 await send(restore, { lock_version: 'bad' }, 422);
 await send(restore, {}, 422, 'POST', `${row}/revisions/999999/restore`);
 await submitForm(restore);
 await expect(mainForm(page).locator('[name="en_markdown"]')).toHaveValue(values.en_markdown);
 await expect(mainForm(page).locator('[name="pl_markdown"]')).toHaveValue(values.pl_markdown);
 await expect(mainForm(page).locator('[name="entity_key"]')).toHaveValue('item:100');
 await submitForm(mainForm(page), { entity_key: '' });
 await expect(mainForm(page).locator('[name="entity_key"]')).toHaveValue('');
 await send(transition('archive'), { lock_version: 'bad' }, 422);
 await submitForm(transition('archive'));
 await expect(page.locator('main > p').first()).toContainText(t(lang, 'Zarchiwizowany', 'Archived'));
 await expect(page.locator('main button')).toHaveCount(0);
 for (const locale of locales) {
  await waitForWikiBudget(`/${locale}/wiki`);
  await visit(anonymous.page, `/${locale}/wiki/articles/${slug}`, 404);
 }
});
