import { test, expect } from '../fixtures.mjs';
import { submit } from '../helpers.mjs';
import { messages, redirect, seedEditor, visit } from '../public-depth-helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';

test('events list links to published localized content and excludes archived events', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const t = messages('templates/events', lang);
  const prefix = `/${lang}/events`;
  const event = `depth-${crypto.randomUUID()}`;
  const title = { en: `Festival ${event}`, pl: `Festiwal ${event}` };
  const body = { en: 'Festival details for visitors.', pl: 'Szczegóły festiwalu dla gości.' };
  test.setTimeout(240_000);
  const editor = await seedEditor(browser, info);
  const detail = `${prefix}/${event}`;
  const status = 'main form[action$="/status"]';
  let publishedEdit;
  try {
    await visit(editor.page, '/admin/events/create', 200, { noStore: false });
    const now = Date.now();
    await submit(editor.page, 'main form', {
      starts_at: new Date(now - 3600_000).toISOString().slice(0, 16),
      ends_at: new Date(now + 86_400_000).toISOString().slice(0, 16),
      featured: true,
      en_slug: event, pl_slug: event, en_title: title.en, pl_title: title.pl,
      en_summary: 'Festival programme.', pl_summary: 'Program festiwalu.',
      en_body: body.en, pl_body: body.pl,
    });
    await expect(editor.page).toHaveURL(/\/admin\/events\/[\da-f-]+\/edit$/);
    await visit(page, detail, 404);
    await editor.page.locator(`${status} select[name=status]`).selectOption('published');
    publishedEdit = new URL(editor.page.url()).pathname;
    await submit(editor.page, status);
    await visit(page, prefix, 200, { lang });
    await expect(page.locator('main h1')).toHaveText(t.events);
    const link = page.locator(`main a[href="${detail}"]`);
    await expect(link).toHaveText(title[lang]);
    await link.click();
    await expect(page).toHaveURL(new RegExp(`/events/${event}$`));
    await expect(page.locator('main h1')).toHaveText(title[lang]);
    await expect(page.locator('main article')).toContainText(body[lang]);
    await visit(page, `/${lang}/today`, 200, { lang });
    const todayLink = page.locator(`section[aria-labelledby=events] a[href="${detail}"]`);
    await expect(todayLink).toHaveText(lang === 'pl' ? 'Zobacz' : 'View');
    await todayLink.click();
    await expect(page.locator('main h1')).toHaveText(title[lang]);

    await redirect(page.request, '/events', '/pl/events');
    await redirect(page.request, `/events/${event}`, `/pl/events/${event}`);
    await visit(page, `${prefix}?page=1`, 200, { lang, message: t.empty });
    await expect(page.locator('main a[href^="?page="]')).toHaveCount(0);
    await visit(page, `${prefix}?page=4294967295`, 200, { lang, message: t.empty });
    await visit(page, `${prefix}?page=4294967296`, 400);
    await visit(page, `${prefix}?page=-1`, 400);
    await visit(page, `${prefix}/Bad--Slug`, 404);
  } finally {
    try {
      if (publishedEdit) {
        await visit(editor.page, publishedEdit, 200, { noStore: false });
        await editor.page.locator(`${status} select[name=status]`).selectOption('archived');
        const [archived] = await Promise.all([
          editor.page.waitForResponse(response => response.request().isNavigationRequest()
            && response.request().method() === 'POST'
            && new URL(response.url()).pathname.endsWith('/status')),
          submit(editor.page, status),
        ]);
        expect(archived.status()).toBe(303);
      }
    } finally {
      await editor.context.close();
    }
  }
  await visit(page, detail, 404);
  await visit(page, prefix, 200, { lang });
  await expect(page.locator(`main a[href="${detail}"]`)).toHaveCount(0);
});

async function withEditorialPages(editor, slug, use) {
  const existing = await query(
    'SELECT e.locale FROM editorial_pages e JOIN pages p ON p.id=e.page_id WHERE p.slug=$1 AND e.locale IN ($2,$3) ORDER BY e.locale',
    [slug, 'en', 'pl'],
  );
  if (existing.length) throw new Error(`Editorial fixture ${slug} requires empty English and Polish revisions`);

  const attempted = [];
  const marker = crypto.randomUUID();
  const save = async (locale, values) => {
    const path = `/admin/pages/${slug}/${locale}`;
    await visit(editor, path, 200, { noStore: false });
    await expect(editor.locator('input[name=expected_revision]'), path).toHaveValue('');
    const body = `${values.body}\n\n<!-- ${marker} -->`;
    attempted.push({ path, locale });
    const [response] = await Promise.all([
      editor.waitForResponse(reply => reply.request().isNavigationRequest()
        && reply.request().method() === 'POST' && new URL(reply.url()).pathname === path),
      submit(editor, 'main form', { ...values, body }),
    ]);
    return response.status();
  };
  try {
    await use(save);
  } finally {
    if (attempted.length) {
      // A rejected form submission can still be navigating on the editor page.
      const cleanup = await editor.context().newPage();
      const errors = [];
      try {
        for (const { path, locale } of attempted.reverse()) {
          try {
            // The audit insert and page save share a transaction. Match its
            // original creation against the current row before using the UI's revision guard.
            const [created] = await query(
              `SELECT e.updated_at::text AS revision
                 FROM editorial_pages e JOIN pages p ON p.id=e.page_id
                WHERE p.slug=$1 AND e.locale=$2 AND strpos(e.body,$3)>0
                  AND EXISTS (
                    SELECT 1 FROM audit_log a
                     WHERE a.xmin=e.xmin AND a.action='cms.page.saved' AND a.target=p.slug
                       AND a.before='null'::jsonb AND a.after->>'locale'=e.locale
                       AND a.after->>'body'=e.body
                  )`,
              [slug, locale, `<!-- ${marker} -->`],
            );
            if (!created) continue;
            await visit(cleanup, path, 200, { noStore: false });
            await expect(cleanup.locator('input[name=expected_revision]'), path).toHaveValue(created.revision);
            const [response] = await Promise.all([
              cleanup.waitForResponse(reply => reply.request().isNavigationRequest()
                && reply.request().method() === 'POST' && new URL(reply.url()).pathname === path),
              cleanup.locator('main button[value=delete]').click(),
            ]);
            expect(response.status(), path).toBe(303);
            await expect(cleanup).toHaveURL(/\/admin\/pages$/);
          } catch (error) {
            errors.push(error);
          }
        }
      } finally {
        await cleanup.close();
      }
      if (errors.length) throw new AggregateError(errors, `Could not clean up editorial fixture ${slug}`);
    }
  }
}

test('server information, help aliases and rules track localized editorial publication', async ({ page, browser }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const t = messages('templates/pages', lang);
  const editor = await seedEditor(browser, info);
  const published = new Date(Date.now() - 60_000).toISOString();
  for (const alias of ['server-information', 'rules', 'support', 'support/report-a-bug']) {
    await redirect(page.request, `/${alias}`, `/pl/${alias}`, 301);
  }
  try {
    for (const [slug, aliases] of [
      ['server-information', ['server-information']],
      ['support', ['help', 'support']],
      ['rules', ['rules']],
      ['report-a-bug', ['support/report-a-bug']],
    ]) {
      const title = { en: `Public information: ${slug}`, pl: `Informacje publiczne: ${slug}` };
      for (const route of aliases) {
        await visit(page, `/${lang}/${route}`, 200, { lang });
        await expect(page.locator('main h1')).toHaveText(t['not-published']);
        await expect(page.locator('main article')).toBeEmpty();
      }
      const link = { en: 'View events', pl: 'Zobacz wydarzenia' };
      await withEditorialPages(editor.page, slug, async save => {
        for (const locale of ['en', 'pl']) {
          expect(await save(locale, {
            title: title[locale], published_at: published,
            body: `## ${title[locale]}\n\n[${link[locale]}](/${locale}/events)`,
          })).toBe(303);
          await expect(editor.page).toHaveURL(/\/admin\/pages$/);
        }
        for (const route of aliases) {
          await visit(page, `/${lang}/${route}`, 200, { lang });
          await expect(page.locator('main h1')).toHaveText(title[lang]);
          await page.locator('main').getByRole('link', { name: link[lang], exact: true }).click();
          await expect(page).toHaveURL(new RegExp(`/${lang}/events$`));
        }
        await visit(page, `/${lang}/pages/${slug}`, 404);
      });
    }
    await visit(page, `/${lang}/pages/Bad--Slug`, 404);
    await visit(page, `/${lang}/pages/missing-depth-editorial`, 404, { lang, message: t.unavailable });
  } finally {
    await editor.context.close();
  }
});

test('editorial cleanup removes English publication after a rejected Polish save', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
  const editor = await seedEditor(browser, info);
  const slug = `depth-partial-${crypto.randomUUID()}`;
  const path = `/en/pages/${slug}`;
  const values = { title: 'Partial editorial fixture', body: 'Temporary publication.' };
  const refused = new Error('Polish publication was refused');
  try {
    await expect(withEditorialPages(editor.page, slug, async save => {
      expect(await save('en', { ...values, published_at: new Date(Date.now() - 60_000).toISOString() })).toBe(303);
      await visit(page, path, 200, { lang: 'en', message: values.body });
      expect(await save('pl', { ...values, published_at: 'invalid' })).toBe(400);
      throw refused;
    })).rejects.toBe(refused);
    await visit(page, path, 404);
    for (const locale of ['en', 'pl']) {
      await visit(editor.page, `/admin/pages/${slug}/${locale}`, 200, { noStore: false });
      await expect(editor.page.locator('input[name=expected_revision]')).toHaveValue('');
    }
  } finally {
    await editor.context.close();
  }
});

test('editorial fixtures refuse to overwrite or delete unpublished revisions', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
  const editor = await seedEditor(browser, info);
  const slug = 'rules';
  const drafts = () => query(
    'SELECT e.locale,e.title,e.body,e.updated_at::text AS revision FROM editorial_pages e JOIN pages p ON p.id=e.page_id WHERE p.slug=$1 ORDER BY e.locale',
    [slug],
  );
  try {
    await withEditorialPages(editor.page, slug, async save => {
      expect(await save('en', { title: 'English draft', body: 'Keep the English draft.' })).toBe(303);
      expect(await save('pl', { title: 'Polski szkic', body: 'Zachowaj polski szkic.' })).toBe(303);
      const before = await drafts();
      expect(before.map(row => row.locale)).toEqual(['en', 'pl']);
      for (const locale of ['en', 'pl']) {
        await visit(page, `/${locale}/${slug}`, 200, {
          lang: locale, message: messages('templates/pages', locale)['not-published'],
        });
      }
      let fixtureStarted = false;
      await expect(withEditorialPages(editor.page, slug, async save => {
        fixtureStarted = true;
        await save('en', { title: 'Replacement', body: 'Attempted replacement.' });
      })).rejects.toThrow('requires empty English and Polish revisions');
      expect(fixtureStarted).toBe(false);
      expect(await drafts()).toEqual(before);
    });
    expect(await drafts()).toEqual([]);
  } finally {
    await editor.context.close();
  }
});

test('Today renders editorial sections and its data and widget refuse missing Game evidence', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const pl = lang === 'pl';
  const t = messages('templates/today', lang);
  for (const route of [`/${lang}/today`, `/today?lang=${lang}`]) {
    await visit(page, route, 200, { lang });
    await expect(page.locator('main h1')).toHaveText(pl ? 'Dziś' : 'Today');
    for (const [id, title] of Object.entries({
      announcements: pl ? 'Ogłoszenia' : 'Announcements',
      events: pl ? 'Wydarzenia' : 'Events',
      news: pl ? 'Aktualności' : 'News',
      liveops: pl ? 'Status świata' : 'World status',
    })) await expect(page.locator(`main h2#${id}`)).toHaveText(title);
  }
  for (const path of [`/${lang}/announcements/ticker`, `/announcements/ticker?lang=${lang}`]) {
    const ticker = await page.request.get(path);
    expect(ticker.status()).toBe(200);
    expect(ticker.headers()['cache-control']).toContain('no-store');
    expect(await ticker.text()).toBe('');
  }
  expect((await page.request.get('/announcements/ticker?lang=invalid')).status()).toBe(404);
  await visit(page, '/today?lang=invalid', 404);
  for (const [prefix, query, status] of [
    [`/${lang}/today`, '', 503],
    ['/today', `?lang=${lang}&world=019b0760-0000-7000-8000-000000000001`, 503],
    [`/${lang}/today`, '?world=invalid', 400],
    [`/${lang}/today`, '?lang=invalid', 400],
    [`/${lang}/today`, '?extra=1', 400],
  ]) {
    const data = await page.request.get(`${prefix}/data${query}`);
    expect(data.status()).toBe(status);
    expect(data.headers()['cache-control']).toContain('no-store');
    expect(await data.json()).toEqual({ status: 'unavailable' });
    const widget = await visit(page, `${prefix}/widget${query}`, status);
    expect(widget.headers()['content-type']).toContain('text/html');
    await expect(page.locator('aside')).toHaveAttribute('aria-label', t.title);
    await expect(page.getByRole('status')).toHaveText(t.unavailable);
    await expect(page.locator('aside dl')).toHaveCount(0);
  }
});

test('wiki editorial article link renders only the reviewed publication', async ({ page, browser }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const editor = await seedEditor(browser, info);
  const slug = `public-guide-${crypto.randomUUID()}`;
  const article = `/${lang}/wiki/articles/${slug}`;
  try {
    await visit(editor.page, '/admin/wiki/articles/create', 200, { noStore: false });
    const title = { en: 'Wiki public guide', pl: 'Publiczny poradnik wiki' };
    const link = { en: 'Try the calculators', pl: 'Wypróbuj kalkulatory' };
    await submit(editor.page, 'main form', {
      en_title: title.en, en_slug: slug, en_summary: 'A public guide.',
      en_markdown: `[${link.en}](/en/wiki/calculators)`,
      pl_title: title.pl, pl_slug: slug, pl_summary: 'Poradnik publiczny.',
      pl_markdown: `[${link.pl}](/pl/wiki/calculators)`,
      note: 'Public depth coverage',
    });
    await expect(editor.page).toHaveURL(/\/admin\/wiki\/articles\/[\da-f-]+\/edit$/);
    await visit(page, article, 404, { noStore: false });
    await submit(editor.page, 'main form[action$="/submit-review"]');
    await visit(page, article, 404, { noStore: false });
    await submit(editor.page, 'main form[action$="/publish"]');
    await visit(page, article, 200, { lang, noStore: false });
    await expect(page.locator('main h1')).toHaveText(title[lang]);
    await page.locator('main').getByRole('link', { name: link[lang], exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`/${lang}/wiki/calculators$`));
    await submit(editor.page, 'main form[action$="/unpublish"]');
    await visit(page, article, 404, { noStore: false });
  } finally {
    await editor.context.close();
  }
});

test('Today and the ticker follow localized announcement publication and action links', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const pl = lang === 'pl';
  const editor = await seedEditor(browser, info);
  const stamp = crypto.randomUUID();
  const now = Date.now();
  const title = pl ? `Ostrzeżenie: Rejestr światów ${stamp}` : `Warning: World registry ${stamp}`;
  const link = pl ? 'Otwórz rejestr światów' : 'Open world registry';
  const body = pl ? 'Sprawdź aktualny katalog światów.' : 'Check the current world catalogue.';
  const today = `/${lang}/today`;
  const ticker = `/${lang}/announcements/ticker`;
  const summary = page.locator('main section[aria-labelledby=announcements]');
  let edit;
  try {
    await visit(editor.page, '/admin/announcements/create', 200, { noStore: false });
    await editor.page.locator('select[name=publication_state]').selectOption('published');
    await editor.page.locator('select[name=severity]').selectOption('warning');
    await submit(editor.page, 'main form', {
      starts_at: new Date(now - 60_000).toISOString(),
      ends_at: new Date(now + 3600_000).toISOString(),
      title: `World registry ${stamp}`, body: 'Check the current world catalogue.',
      action_label: 'Open world registry', action_url: '/worlds',
      title_pl: `Rejestr światów ${stamp}`, body_pl: 'Sprawdź aktualny katalog światów.',
      action_label_pl: 'Otwórz rejestr światów',
      translation_published_at: new Date(now - 60_000).toISOString(),
    });
    await expect(editor.page).toHaveURL(/\/admin\/announcements\/[\da-f-]+\/edit$/);
    edit = new URL(editor.page.url()).pathname;
    await visit(page, today, 200, { lang });
    const card = summary.locator('article').filter({ has: page.getByRole('heading', { name: title, exact: true }) });
    await expect(card).toContainText(body);
    const action = card.getByRole('link', { name: link, exact: true });
    await expect(action).toHaveAttribute('href', '/worlds');
    await action.click();
    await expect(page).toHaveURL(/\/worlds$/);
    await expect(page.locator('main h1')).toHaveText(pl ? 'Status światów' : 'World status');

    await visit(page, ticker, 200);
    const tickerCard = page.locator('aside article').filter({ has: page.getByRole('heading', { name: title, exact: true }) });
    await expect(tickerCard).toContainText(body);
    await expect(tickerCard.getByRole('link', { name: link, exact: true })).toHaveAttribute('href', '/worlds');
  } finally {
    if (edit) {
      await visit(editor.page, edit, 200, { noStore: false });
      await editor.page.locator('select[name=publication_state]').selectOption('draft');
      await submit(editor.page);
      await expect(editor.page).toHaveURL(new RegExp(`${edit}$`));
      const response = await page.request.get(ticker);
      expect(response.status()).toBe(200);
      expect(await response.text()).not.toContain(stamp);
    }
    await editor.context.close();
  }
});
