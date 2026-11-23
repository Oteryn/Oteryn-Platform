import { escapeRegex } from '../regex.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import {
 test, expect, editPath, formFor, key, labels, link, locales, localized as t,
 mainForm, refuse, request, send, stamp, submitForm, title, visit,
} from '../content-helpers.mjs';

test('news draft, translation review, public links, update and both delete methods', async ({content}, {project}) => {
 const { anonymous, editor } = content;
 const page = editor.page;
 const lang = project.metadata.locale;
 const slug = key('editorial-news');
 const titles = { en: `News ${slug}`, pl: `Aktualność ${slug}` };
 const create = '/admin/news';
 await refuse(content, create);
 await refuse(content, `${create}/create`);
 await refuse(content, create, 'POST', { slug, title: titles.en, body: 'English text.' });
 await visit(page, `${create}?lang=${lang}`);
 await title(page, t(lang, 'Aktualności', 'News'), lang);
 await link(page, `${create}/create?lang=${lang}`, t(lang, 'Nowy wpis', 'New post'));
 for (const [name, label] of [
  ['slug', t(lang, 'Adres', 'Slug') + ' (en)'],
  ['title', t(lang, 'Tytuł', 'Title') + ' (en)'],
  ['title_pl', t(lang, 'Tytuł', 'Title') + ' (pl)'],
  ['body', t(lang, 'Treść', 'Body') + ' (en)'],
  ['body_pl', t(lang, 'Treść', 'Body') + ' (pl)'],
  ['published_at', t(lang, 'Data publikacji (UTC)', 'Publication date (UTC)') + ' (en)'],
  ['translation_published_at', t(lang, 'Data publikacji (UTC)', 'Publication date (UTC)') + ' (pl)'],
 ]) {
  await expect(page.getByLabel(label, { exact: true })).toHaveAttribute('name', name);
 }
 await expect(mainForm(page).getByRole('button')).toHaveText(t(lang, 'Zapisz', 'Save'));
 await send(mainForm(page), { slug: 'INVALID SLUG', title: titles.en, body: 'English text.' }, 422);
 await submitForm(mainForm(page), {
  slug, title: titles.en, body: 'English text.',
  title_pl: titles.pl, body_pl: 'Polska treść.',
 });
 await expect(page).toHaveURL(/\/admin\/news\/[a-f0-9-]+\/edit$/);
 const edit = editPath(page);
 const row = edit.slice(0, -5);
 await title(page, t(lang, 'Aktualności', 'News'), lang);
 await expect(formFor(page, '/delete').getByRole('button')).toHaveText(t(lang, 'Usuń', 'Delete'));
 for (const locale of locales) await visit(anonymous.page, `/${locale}/news/${slug}`, 404);
 await refuse(content, edit);
 for (const method of ['POST', 'PUT', 'DELETE']) await refuse(content, row, method);
 await refuse(content, `${row}/delete`, 'POST');
 await visit(page, `${create}?lang=${lang}`);
 await link(page, `${edit}?lang=${lang}`, titles.en);
 await send(mainForm(page), { published_at: 'bad date' }, 422);
 await submitForm(mainForm(page), { published_at: stamp(-10), translation_published_at: stamp(-9) });
 for (const locale of locales) {
  await visit(anonymous.page, `/${locale}/news`);
  await link(anonymous.page, `/${locale}/news/${slug}`, titles[locale]);
  await title(anonymous.page, titles[locale], locale);
  await expect(anonymous.page.locator('main')).toContainText(t(locale, 'Polska treść.', 'English text.'));
 }
 await visit(anonymous.page, `/news?lang=${lang}`);
 await visit(anonymous.page, `/news/${slug}?lang=${lang}`);
 await title(anonymous.page, titles[lang], lang);
 await request(page, row, 'PUT', 403, { form: {} });
 const stale = await mainForm(page).locator('[name="expected_revision"]').inputValue();
 await send(mainForm(page), { body: 'Updated English text.' }, 302, 'PUT');
 await send(mainForm(page), { expected_revision: stale, body: 'Stale editor text.' }, 409);
 await visit(anonymous.page, `/pl/news/${slug}`, 404);
 await visit(anonymous.page, `/en/news/${slug}`);
 await expect(anonymous.page.locator('main')).toContainText('Updated English text.');
 await page.reload();
 await submitForm(mainForm(page), { body_pl: 'Zaktualizowana treść.', translation_published_at: stamp(-8) });
 await visit(anonymous.page, `/pl/news/${slug}`);
 await expect(anonymous.page.locator('main')).toContainText('Zaktualizowana treść.');
 await submitForm(formFor(page, '/delete'));
 for (const locale of locales) await visit(anonymous.page, `/${locale}/news/${slug}`, 404);
 await visit(page, `${create}/create`);
 await submitForm(mainForm(page), { slug, title: titles.en, body: 'A second draft.' });
 const second = editPath(page).slice(0, -5);
 await request(page, second, 'DELETE', 403);
 await send(formFor(page, '/delete'), {}, 302, 'DELETE', second);
 await visit(page, `${second}/edit`, 404);
});

test('events draft, featured edit, schedule, publish and archive', async ({content}, {project}) => {
 const { anonymous, editor } = content;
 const page = editor.page;
 const lang = project.metadata.locale;
 const slug = key('event');
 const create = '/admin/events';
 const reason = 'Exercise the editorial event workflow';
 const values = {
  reason,
  starts_at: stamp(60).slice(0, 16), ends_at: stamp(120).slice(0, 16),
  en_title: `Event ${slug}`, en_slug: slug, en_summary: 'English summary.', en_body: 'English event details.',
  pl_title: `Wydarzenie ${slug}`, pl_slug: `pl-${slug}`, pl_summary: 'Polskie streszczenie.', pl_body: 'Polskie szczegóły.',
 };
 await refuse(content, create);
 await refuse(content, `${create}/create`);
 await refuse(content, create, 'POST', values);
 await visit(page, create);
 await title(page, t(lang, 'Wydarzenia', 'Events'), lang);
 await link(page, `${create}/create`, t(lang, 'Utwórz wydarzenie', 'Create event'));
 await labels(mainForm(page), lang, [
  ['starts_at', 'Początek (UTC)', 'Starts at (UTC)'],
  ['ends_at', 'Koniec (UTC)', 'Ends at (UTC)'],
  ['featured', 'Wyróżnione', 'Featured'],
  ['reason', 'Powód zmiany', 'Reason for change'],
 ]);
 for (const locale of locales) {
  await labels(mainForm(page).getByRole('group', {name: t(locale, 'Polski', 'English')}), lang, [
   [`${locale}_title`, 'Tytuł', 'Title'], [`${locale}_slug`, 'Adres', 'Slug'],
   [`${locale}_summary`, 'Podsumowanie', 'Summary'], [`${locale}_body`, 'Treść', 'Content'],
  ]);
 }
 await expect(mainForm(page).getByRole('button')).toHaveText(t(lang, 'Zapisz szkic', 'Save draft'));
 await send(mainForm(page), { ...values, reason: '' }, 422);
 await send(mainForm(page), { ...values, starts_at: values.ends_at }, 422);
 await submitForm(mainForm(page), values);
 await expect(page).toHaveURL(/\/admin\/events\/[a-f0-9-]+\/edit$/);
 const edit = editPath(page);
 const row = edit.slice(0, -5);
 await refuse(content, edit);
 for (const method of ['POST', 'PUT']) {
  await refuse(content, row, method, values);
  await refuse(content, `${row}/status`, method, { lock_version: '1', status: 'published', reason });
 }
 await visit(page, create);
 await link(page, edit);
 await send(mainForm(page), { ends_at: 'bad date', reason }, 422);
 await submitForm(mainForm(page), { featured: true, reason });
 await expect(mainForm(page).getByLabel(t(lang, 'Wyróżnione', 'Featured'))).toBeChecked();
 await request(page, row, 'PUT', 403, { form: {} });
 await send(mainForm(page), { ends_at: 'bad date', reason }, 422, 'PUT');
 await send(mainForm(page), { en_summary: 'Revised English summary.', reason }, 303, 'PUT');
 await page.reload();
 await expect(mainForm(page).locator('[name="en_summary"]')).toHaveValue('Revised English summary.');
 const status = formFor(page, '/status');
 await expect(status.getByLabel(t(lang, 'Status publikacji', 'Publication status')).locator('option'))
  .toHaveText([
   t(lang, 'Szkic', 'Draft'), t(lang, 'Zaplanowane', 'Scheduled'),
   t(lang, 'Opublikowane', 'Published'), t(lang, 'Zarchiwizowane', 'Archived'),
  ]);
 await expect(status.getByRole('button')).toHaveText(t(lang, 'Zmień status', 'Change status'));
 await send(status, { status: 'unknown', reason }, 422);
 await request(page, `${row}/status`, 'PUT', 403, { form: {} });
 await send(status, { status: 'unknown', reason }, 422, 'PUT');
 await send(status, { status: 'draft', reason }, 303, 'PUT');
 await page.reload();
 await submitForm(status, { status: 'scheduled', reason });
 await expect(page.locator('main > p').first()).toHaveText(t(lang, 'Zaplanowane', 'Scheduled'));
 await submitForm(status, { status: 'draft', reason });
 await expect(page.locator('main > p').first()).toHaveText(t(lang, 'Szkic', 'Draft'));
 await submitForm(mainForm(page), { starts_at: stamp(-60).slice(0, 16), reason });
 await send(status, { status: 'scheduled', reason }, 409);
 await send(status, { status: 'published', lock_version: '1', reason }, 409);
 for (const locale of locales) await visit(anonymous.page, `/${locale}/events/${values[`${locale}_slug`]}`, 404);
 await submitForm(status, { status: 'published', reason });
 for (const locale of locales) {
  await visit(anonymous.page, `/${locale}/events`);
  await link(anonymous.page, `/${locale}/events/${values[`${locale}_slug`]}`, values[`${locale}_title`]);
  await title(anonymous.page, values[`${locale}_title`], locale);
  await expect(anonymous.page.locator('main')).toContainText(values[`${locale}_body`]);
 }
 await request(anonymous.page, '/events', 'GET', 303);
 for (const [source, preferred] of [['pl', 'en'], ['en', 'pl']]) {
  const response = await anonymous.page.request.get(`/events/${values[`${source}_slug`]}`, {
   headers: { 'Accept-Language': preferred }, maxRedirects: 0,
  });
  expect(response.status()).toBe(303);
  expect(response.headers().location).toBe(`/${preferred}/events/${values[`${preferred}_slug`]}`);
  await visit(anonymous.page, response.headers().location);
 }
 await visit(anonymous.page, '/events/INVALID', 404);
 await submitForm(status, { status: 'archived', reason });
 for (const locale of locales) await visit(anonymous.page, `/${locale}/events/${values[`${locale}_slug`]}`, 404);
});

const legalPages = [
 ['terms-of-service', 'terms'], ['privacy-policy', 'privacy'], ['cookie-policy', 'cookies'],
];

for (const [slug, route] of legalPages) {
 const legalTest = test.extend({
  restorePublishedEditions: [async ({}, use) => {
   const [{ editions }] = await query(`SELECT jsonb_agg(to_jsonb(e) ORDER BY e.locale) AS editions
    FROM editorial_pages e JOIN pages p ON p.id=e.page_id WHERE p.slug=$1`, [slug]);
   expect(editions?.map(({locale}) => locale)).toEqual(['en', 'pl']);
   try {
    await use();
   } finally {
    // Signup and the default-page checks use the original live editions.
    // Restore their editable rows and preserve all immutable publication history.
    const restored = await query(`INSERT INTO editorial_pages
     SELECT * FROM jsonb_populate_recordset(NULL::editorial_pages, $1::jsonb)
     ON CONFLICT (page_id, locale) DO UPDATE SET
      title=EXCLUDED.title, body=EXCLUDED.body, published_at=EXCLUDED.published_at,
      source_revision=EXCLUDED.source_revision, revision=EXCLUDED.revision,
      legal_version=EXCLUDED.legal_version, legal_effective_date=EXCLUDED.legal_effective_date,
      updated_at=EXCLUDED.updated_at
     RETURNING locale`, [JSON.stringify(editions)]);
    expect(restored.map(({locale}) => locale).sort()).toEqual(['en', 'pl']);
   }
  }, {auto: true}],
 });
 legalTest(`legal ${route}: immutable versions, stale translation refusal and reapproval`, async ({content}, {project}) => {
  const { anonymous, editor, admin } = content;
  const page = editor.page;
  const lang = project.metadata.locale;
  const listing = '/admin/pages';
  const en = `${listing}/${slug}/en`;
  const pl = `${listing}/${slug}/pl`;
  const publicPath = locale => `/${locale}/legal/${route}`;
  const version = key('v');
  const values = {
   title: `English legal text ${version}`, body: '## English policy',
   published_at: stamp(-5), legal_version: version, legal_effective_date: '2026-01-01',
  };
  const translation = { ...values, title: `Polski dokument ${version}`, body: '## Polska polityka' };
  const save = 'button[value="save"]';
  const unpublished = 'Ta strona nie została jeszcze opublikowana.';
  await refuse(content, listing);
  for (const target of [en, pl]) {
   await refuse(content, target);
   await refuse(content, target, 'POST', { ...values, action: 'save' });
  }
  await visit(page, listing);
  await title(page, t(lang, 'Strony', 'Pages'), lang);
  await expect(page.getByLabel(t(lang, 'Adres strony', 'Page slug'))).toBeVisible();
  const form = mainForm(page);
  await form.locator('[name="slug"]').fill(slug);
  await form.locator('[name="locale"]').selectOption('en');
  await form.getByRole('button', {name: t(lang, 'Utwórz lub edytuj', 'Create or edit')}).click();
  await expect(page).toHaveURL(new RegExp(`${escapeRegex(en)}$`));
  await send(mainForm(page), { ...values, legal_version: '', action: 'save' }, 400);
  await submitForm(mainForm(page), values, save);
  await link(page, en);
  await expect(page.getByLabel('Legal version')).toHaveValue(version);
  await visit(anonymous.page, publicPath('en'));
  await title(anonymous.page, values.title, 'en');
  await expect(anonymous.page.locator('main')).toContainText(`${version} 2026-01-01`);
  await visit(anonymous.page, publicPath('pl'));
  await title(anonymous.page, unpublished, 'pl');
  await visit(page, pl);
  await labels(mainForm(page), 'pl', [
   ['title', 'Tytuł', 'Title'], ['body', 'Treść (Markdown)', 'Content (Markdown)'],
   ['published_at', 'Data publikacji (RFC 3339; puste dla szkicu)', 'Publication time (RFC 3339; empty for draft)'],
   ['legal_version', 'Wersja dokumentu prawnego', 'Legal version'], ['legal_effective_date', 'Data obowiązywania', 'Effective date'],
  ]);
  await expect(mainForm(page).getByRole('button')).toHaveText(['Zapisz', 'Usuń']);
  await submitForm(mainForm(page), translation, save);
  await visit(anonymous.page, publicPath('pl'));
  await title(anonymous.page, translation.title, 'pl');
  await expect(anonymous.page.getByRole('heading', {name: 'Polska polityka'})).toBeVisible();

  await link(page, pl);
  const stale = mainForm(page);
  const oldSource = await stale.locator('[name="reviewed_source_revision"]').inputValue();
  await visit(admin.page, en);
  await send(mainForm(admin.page), { body: 'Changed under the same version', action: 'save' }, 409);
  const deletion = admin.page.waitForResponse(response => response.request().method() === 'POST'
   && new URL(response.url()).pathname === en && response.status() === 409);
  const deletedPage = admin.page.waitForEvent('framenavigated', frame => frame === admin.page.mainFrame());
  await submitForm(mainForm(admin.page), {}, 'button[value="delete"]');
  expect((await deletion).status()).toBe(409);
  await deletedPage;
  await visit(admin.page, en);
  await submitForm(mainForm(admin.page), {
   body: '## Updated policy', legal_version: `${version}-2`, legal_effective_date: '2026-02-01',
  }, save);
  await visit(anonymous.page, publicPath('pl'));
  await title(anonymous.page, unpublished, 'pl');
  await visit(anonymous.page, publicPath('en'));
  await expect(anonymous.page.getByRole('heading', {name: 'Updated policy'})).toBeVisible();
  await expect(anonymous.page.locator('main')).toContainText(`${version}-2 2026-02-01`);
  await send(stale, { action: 'save' }, 409);
  await visit(page, pl);
  await expect(mainForm(page).locator('[name="reviewed_source_revision"]')).not.toHaveValue(oldSource);
  await submitForm(mainForm(page), {
   body: '## Zmieniona polityka', legal_version: `${version}-2`, legal_effective_date: '2026-02-01',
  }, save);
  await visit(anonymous.page, publicPath('pl'));
  await title(anonymous.page, translation.title, 'pl');
  await expect(anonymous.page.locator('main')).toContainText(`${version}-2 2026-02-01`);
  await expect(anonymous.page.getByRole('heading', {name: 'Zmieniona polityka'})).toBeVisible();
  await visit(anonymous.page, `/${lang}/pages/${slug}`, 404);
 });
}

test('custom page requires an English source and can be deleted through the form', async ({content}) => {
 const { anonymous, editor } = content;
 const page = editor.page;
 const slug = key('custom');
 const en = `/admin/pages/${slug}/en`;
 const pl = `/admin/pages/${slug}/pl`;
 const values = { title: 'Content page', body: '## Custom page', published_at: stamp(-5) };
 await visit(page, '/admin/pages?slug=INVALID&locale=de', 400);
 await visit(page, `/admin/pages/${slug}/de`, 400);
 await visit(page, pl);
 await send(mainForm(page), { ...values, action: 'save' }, 409);
 await visit(page, en);
 await submitForm(mainForm(page), values, 'button[value="save"]');
 await visit(page, pl);
 await submitForm(mainForm(page), { ...values, title: 'Strona', body: '## Treść strony' }, 'button[value="save"]');
 for (const locale of locales) {
  await visit(anonymous.page, `/${locale}/pages/${slug}`);
  await title(anonymous.page, t(locale, 'Strona', 'Content page'), locale);
 }
 await link(page, en);
 await submitForm(mainForm(page), {}, 'button[value="delete"]');
 for (const locale of locales) {
  await visit(anonymous.page, `/${locale}/pages/${slug}`, 404);
  await expect(anonymous.page.locator('main h1')).toHaveText(t(locale,
   'Ta strona jest obecnie niedostępna.', 'This page is currently unavailable.'));
 }
 for (const [route, reserved, helpTitle] of [
  ['rules', 'rules'],
  ['help', 'support', ['Pomoc dla graczy', 'Player support']],
  ['support', 'support', ['Pomoc dla graczy', 'Player support']],
  ['server-information', 'server-information'],
  ['support/report-a-bug', 'report-a-bug', ['Zgłoś błąd', 'Report a bug']],
 ]) {
  for (const locale of locales) {
   await visit(anonymous.page, `/${locale}/${route}`);
   const heading = helpTitle ? t(locale, ...helpTitle) : t(locale,
    'Ta strona nie została jeszcze opublikowana.', 'This page has not been published yet.');
   await title(anonymous.page, heading, locale);
   if (helpTitle) {
    for (const [path, pl, en] of [
     ['support/tickets/create', 'Wyślij zgłoszenie do pomocy', 'Submit a support ticket'],
     ['forgot-password', 'Odzyskaj dostęp do konta', 'Recover access to your account'],
    ]) {
     await expect(anonymous.page.getByRole('link', {name: t(locale, pl, en), exact: true}))
      .toHaveAttribute('href', `/${locale}/${path}`);
    }
   }
   await visit(anonymous.page, `/${locale}/pages/${reserved}`, 404);
  }
 }
});
