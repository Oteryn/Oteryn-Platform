import { expect, get, ready, submit, test, unique, visit } from './helpers.mjs';

const languages = ['en', 'pl'];
const prior = () => new Date(Date.now() - 5_000).toISOString();
const future = () => new Date(Date.now() + 600_000).toISOString();
const main = 'main form';
const article = 'main article';
const heading = page => page.getByRole('heading', { level: 1 });
const pageFields = (title, body, published_at = '') => ({ title, body, published_at });
const newsFields = (slug, title, body, publication = '') => ({
  slug, title: title.en, title_pl: title.pl, body: body.en, body_pl: body.pl,
  published_at: publication, translation_published_at: publication,
});
const tokens = {
  en: {
    news: 'News', newPost: 'New post', pages: 'Pages', addPage: 'Create or edit',
    pageUnavailable: 'This page is currently unavailable.',
    wikiMissing: 'Article not found.', wikiHistory: 'Revision history',
    wikiDraft: 'Status: Draft', wikiReview: 'Status: In review', wikiLive: 'Status: Published',
    wikiConflict: 'This article changed or the requested transition is unavailable. Reload before continuing.',
    wikiInvalid: 'Check the input. Search requires 2–80 characters and a page from 1 to 1000.',
  },
  pl: {
    news: 'Aktualności', newPost: 'Nowy wpis', pages: 'Strony', addPage: 'Utwórz lub edytuj',
    pageUnavailable: 'Ta strona jest obecnie niedostępna.',
    wikiMissing: 'Nie znaleziono artykułu.', wikiHistory: 'Historia wersji',
    wikiDraft: 'Stan: Szkic', wikiReview: 'Stan: W przeglądzie', wikiLive: 'Stan: Opublikowany',
    wikiConflict: 'Artykuł zmienił się lub ta zmiana stanu jest niedostępna. Odśwież stronę.',
    wikiInvalid: 'Sprawdź dane. Wyszukiwanie wymaga 2–80 znaków i strony od 1 do 1000.',
  },
};

test.beforeEach(async ({ context }) => ready(context.request));

async function editNews(page, path, locale) {
  await visit(page, `${path}?lang=${locale}`);
  await expect(heading(page)).toHaveText(tokens[locale].news);
}

async function newNews(page, locale) {
  await editNews(page, '/admin/news', locale);
  await page.getByRole('link', { name: tokens[locale].newPost }).click();
  await expect(page).toHaveURL(new RegExp(`/admin/news/create\\?lang=${locale}$`));
}

async function hideNews(page, edit, slug) {
  await visit(page, edit);
  await expect(page.locator('[name=slug]')).toHaveValue(slug);
  expect(await submit(page, 'main form[action$="/delete"]'), 'Remove this run’s news post').toBe(302);
}

async function bothHidden(visitor, stem, slug) {
  for (const locale of languages) await visit(visitor, `/${locale}/${stem}/${slug}`, 404);
}

test('news drafts, schedules, publishes, edits and unpublishes in both languages', async ({
  page: visitor, staffPage,
}, info) => {
  const { locale } = info.project.metadata;
  const { context, page } = await staffPage('editor');
  const slug = `sim-content-editor-news-${unique()}`;
  const title = { en: `Staging news ${slug}`, pl: `Aktualności testowe ${slug}` };
  const body = {
    en: 'First English edition.\n\n**Editorial notes** <em>HTML sample</em>',
    pl: 'Pierwsza polska wersja.\n\n**Notatki redakcyjne** <em>Przykład HTML</em>',
  };
  const revision = { en: 'Revised English edition.', pl: 'Zaktualizowana polska wersja.' };
  let edit;
  try {
    await newNews(page, locale);
    expect(await submit(page, main, newsFields(slug, title, body))).toBe(302);
    await expect(page).toHaveURL(/\/admin\/news\/[a-f0-9-]+\/edit$/);
    edit = new URL(page.url()).pathname;
    await editNews(page, edit, locale);
    await expect(page.locator('[name=published_at]')).toHaveValue('');
    await expect(page.locator('[name=body_pl]')).toHaveValue(body.pl);
    await bothHidden(visitor, 'news', slug);

    const scheduled = future();
    expect(await submit(page, main, { published_at: scheduled, translation_published_at: scheduled })).toBe(302);
    await editNews(page, edit, locale);
    for (const field of ['published_at', 'translation_published_at']) {
      expect(new Date(await page.locator(`[name=${field}]`).inputValue()).toISOString()).toBe(scheduled);
    }
    await bothHidden(visitor, 'news', slug);

    const publication = prior();
    expect(await submit(page, main, {
      published_at: publication, translation_published_at: publication,
    })).toBe(302);
    for (const language of [locale, ...languages.filter(value => value !== locale)]) {
      await visit(visitor, `/${language}/news`);
      await visitor.getByRole('link', { name: title[language], exact: true }).click();
      await expect(visitor).toHaveURL(new RegExp(`/${language}/news/${slug}$`));
      await expect(heading(visitor)).toHaveText(title[language]);
      await expect(visitor.locator(article)).toHaveText(body[language].replaceAll('\n', ' '), { useInnerText: true });
      await expect(visitor.locator(`${article} em, ${article} strong`)).toHaveCount(0);
    }

    const stale = await context.newPage();
    try {
      await editNews(stale, edit, locale);
      await editNews(page, edit, locale);
      expect(await submit(page, main, { body: revision.en, body_pl: revision.pl })).toBe(302);
      expect(await submit(stale, main, { body: 'Discard this stale update.' })).toBe(409);
      await editNews(stale, edit, locale);
      await expect(stale.locator('[name=body]')).toHaveValue(revision.en);
    } finally {
      await stale.close();
    }
    for (const language of languages) {
      await visit(visitor, `/${language}/news/${slug}`);
      await expect(heading(visitor)).toHaveText(title[language]);
      await expect(visitor.locator(article)).toHaveText(revision[language]);
    }
    await editNews(page, edit, locale);
    expect(await submit(page, main, { published_at: '', translation_published_at: '' })).toBe(302);
    await bothHidden(visitor, 'news', slug);
    await editNews(page, edit, locale);
    await expect(page.locator('[name=published_at]')).toHaveValue('');
  } finally {
    try {
      if (edit) await hideNews(page, edit, slug);
    } finally {
      await context.close();
    }
  }
});

test('news form rejects empty fields, duplicate slug and overlong body', async ({ page: visitor, staffPage }, info) => {
  const { locale } = info.project.metadata;
  const { context, page } = await staffPage('editor');
  const slug = `sim-content-editor-validation-${unique()}`;
  const title = { en: 'Validation draft', pl: 'Szkic do walidacji' };
  const body = { en: 'English draft body.', pl: 'Treść polskiego szkicu.' };
  const input = newsFields(slug, title, body);
  let edit;
  try {
    for (const override of [{ title: '' }, { slug: '' }, { body: 'x'.repeat(100_001) }]) {
      await newNews(page, locale);
      expect(await submit(page, main, { ...input, ...override }), Object.keys(override)[0]).toBe(422);
      await expect(page).toHaveURL(new RegExp(`/admin/news\\?lang=${locale}$`));
    }
    await newNews(page, locale);
    expect(await submit(page, main, input)).toBe(302);
    await expect(page).toHaveURL(/\/admin\/news\/[a-f0-9-]+\/edit$/);
    edit = new URL(page.url()).pathname;
    await newNews(page, locale);
    expect(await submit(page, main, { ...input, title: 'Would replace the draft' })).toBe(422);
    await editNews(page, edit, locale);
    await expect(page.locator('[name=title]')).toHaveValue(title.en);
    await bothHidden(visitor, 'news', slug);
  } finally {
    try {
      if (edit) await hideNews(page, edit, slug);
    } finally {
      await context.close();
    }
  }
});

test('localized CMS pages publish only a current translation', async ({ page: visitor, staffPage }, info) => {
  const { locale } = info.project.metadata;
  const { context, page } = await staffPage('editor');
  const slug = `sim-content-editor-page-${unique()}`;
  const path = language => `/admin/pages/${slug}/${language}`;
  const title = { en: `Staging page ${slug}`, pl: `Strona testowa ${slug}` };
  const content = { en: 'English page text.', pl: 'Polska treść strony.' };
  const body = language => `## ${content[language]}\n\n<iframe></iframe>\n\n[Wiki](/${language}/wiki)`;
  const save = fields => submit(page, main, fields, 'button[value=save]');
  const saved = new Set();
  try {
    await visit(page, '/admin/pages');
    await expect(heading(page)).toHaveText(tokens[locale].pages);
    await page.locator('[name=locale]').selectOption('en');
    await page.locator('[name=slug]').fill(slug);
    await page.getByRole('button', { name: tokens[locale].addPage }).click();
    await expect(page).toHaveURL(new RegExp(`${path('en')}$`));
    const publish = prior();
    expect(await save(pageFields(title.en, body('en'), 'tomorrow'))).toBe(400);
    await visit(page, path('pl'));
    expect(await save(pageFields(title.pl, body('pl'), publish))).toBe(409);
    await visit(page, path('en'));
    expect(await save(pageFields(title.en, body('en')))).toBe(303);
    saved.add('en');
    await visit(visitor, `/${locale}/pages/${slug}`, 404);
    await expect(heading(visitor)).toHaveText(tokens[locale].pageUnavailable);
    await visit(page, path('en'));
    expect(await save(pageFields(title.en, body('en'), publish))).toBe(303);
    await visit(page, path('pl'));
    expect(await save(pageFields(title.pl, body('pl'), publish))).toBe(303);
    saved.add('pl');
    for (const language of languages) {
      await visit(visitor, `/${language}/pages/${slug}`);
      await expect(heading(visitor)).toHaveText(title[language]);
      await expect(visitor.locator(article).getByRole('heading', { level: 2 })).toHaveText(content[language]);
      await expect(visitor.locator(article).getByRole('link', { name: 'Wiki' })).toHaveAttribute('href', `/${language}/wiki`);
      await expect(visitor.locator(`${article} iframe`)).toHaveCount(0);
    }
    await visit(page, path('en'));
    expect(await save({ body: 'New English revision.' })).toBe(303);
    await visit(visitor, `/pl/pages/${slug}`, 404);
    await expect(heading(visitor)).toHaveText(tokens.pl.pageUnavailable);
    await visit(visitor, `/en/pages/${slug}`);
    await expect(visitor.locator(article)).toHaveText('New English revision.');
    await visit(page, path('en'));
    expect(await save({ published_at: '' })).toBe(303);
    await bothHidden(visitor, 'pages', slug);
  } finally {
    try {
      for (const language of [...saved].reverse()) {
        await visit(page, path(language));
        await expect(page.locator('[name=title]')).toHaveValue(title[language]);
        expect(await submit(page, main, {}, 'button[value=delete]')).toBe(303);
      }
    } finally {
      await context.close();
    }
  }
});

test('wiki previews drafts, refuses a stale review and retains revision history', async ({
  page: visitor, staffPage,
}, info) => {
  const { locale } = info.project.metadata;
  const { context, page } = await staffPage('editor');
  const slug = `sim-content-editor-guide-${unique()}`;
  const title = { en: `Staging guide ${slug}`, pl: `Poradnik testowy ${slug}` };
  const content = { en: 'How to read the English guide', pl: 'Jak czytać polski poradnik' };
  const input = {
    en_title: title.en, pl_title: title.pl, en_slug: slug, pl_slug: slug,
    en_summary: 'English guide summary.', pl_summary: 'Streszczenie polskiego poradnika.',
    en_markdown: `## ${content.en}`, pl_markdown: `## ${content.pl}`,
    note: 'First bilingual draft',
  };
  const form = action => `main form[action$="/${action}"]`;
  const change = (tab, action) => submit(tab, form(action));
  let edit;
  try {
    await visit(page, '/admin/wiki/articles/create');
    expect(await submit(page, main, { ...input, en_markdown: '<mark>Example HTML</mark>' })).toBe(422);
    await expect(page.locator('main')).toContainText(tokens[locale].wikiInvalid);
    await visit(page, '/admin/wiki/articles/create');
    expect(await submit(page, main, input)).toBe(303);
    await expect(page).toHaveURL(/\/admin\/wiki\/articles\/[a-f0-9-]+\/edit$/);
    edit = new URL(page.url()).pathname;
    await expect(page.locator('main')).toContainText(tokens[locale].wikiDraft);
    for (const language of languages) {
      await visit(visitor, `/${language}/wiki/articles/${slug}`, 404);
      await expect(visitor.locator('main')).toContainText(tokens[language].wikiMissing);
      await visit(page, edit);
      await page.locator(`main a[href$="/preview/${language}"]`).click();
      await expect(heading(page)).toHaveText(title[language]);
      await expect(page.locator(article).getByRole('heading', { level: 2 })).toHaveText(content[language]);
    }
    await visit(page, edit);
    const stale = await context.newPage();
    try {
      await visit(stale, edit);
      expect(await change(page, 'submit-review')).toBe(303);
      await expect(page.locator('main')).toContainText(tokens[locale].wikiReview);
      expect(await change(stale, 'submit-review')).toBe(409);
      await expect(stale.locator('main')).toContainText(tokens[locale].wikiConflict);
    } finally {
      await stale.close();
    }
    expect(await change(page, 'publish')).toBe(303);
    await expect(page.locator('main')).toContainText(tokens[locale].wikiLive);
    for (const language of languages) {
      await visit(visitor, `/${language}/wiki/articles/${slug}`);
      await expect(heading(visitor)).toHaveText(title[language]);
      await expect(visitor.locator(article).getByRole('heading', { level: 2 })).toHaveText(content[language]);
    }
    expect(await change(page, 'unpublish')).toBe(303);
    await bothHidden(visitor, 'wiki/articles', slug);
    expect(await submit(page, main, {
      en_markdown: '## Updated English guide', pl_markdown: '## Zaktualizowany poradnik',
      note: 'Second bilingual draft',
    })).toBe(303);
    await page.locator('main a[href$="/revisions"]').click();
    await expect(heading(page)).toHaveText(tokens[locale].wikiHistory);
    for (const note of [input.note, 'Second bilingual draft']) {
      await expect(page.getByText(note, { exact: true })).toHaveCount(2);
    }
    await bothHidden(visitor, 'wiki/articles', slug);
    const audit = `/admin/console/audit?lang=${locale}`;
    const anonymous = await get(visitor.request, audit);
    if (anonymous.status() === 404) await visit(page, audit, 404);
    else {
      expect(anonymous.status()).toBe(303);
      await visit(page, audit, 403);
    }
  } finally {
    try {
      if (edit) {
        await visit(page, edit);
        expect(await change(page, 'archive')).toBe(303);
      }
    } finally {
      await context.close();
    }
  }
});
