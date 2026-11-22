import {randomUUID} from 'node:crypto';
import {test, expect} from '../fixtures.mjs';
import {submit} from '../helpers.mjs';
import {action, isolatedAdmin, query, role} from '../staff-rbac-helpers.mjs';

test('staff creates a page through the finder, publishes it and deletes it', async ({browser}, info) => {
  test.setTimeout(300_000);
  const {context, page} = await isolatedAdmin(browser, info);
  try {
    const slug = `ledger-${randomUUID()}`;
    const title = 'Ledger editorial page';
    const edit = `/admin/pages/${slug}/en`;
    const publicPage = `/en/pages/${slug}`;
    expect((await page.goto('/admin/pages')).status()).toBe(200);
    const finder = page.locator('main form[method=get]');
    await finder.locator('[name=slug]').fill(slug);
    await finder.locator('[name=locale]').selectOption('en');
    await finder.getByRole('button').click();
    await expect(page).toHaveURL(new RegExp(`${edit}$`));
    const form = page.locator('main form[method=post]');
    await form.locator('[name=title]').fill(title);
    await form.locator('[name=body]').fill('Published through the editorial browser.');
    await form.locator('[name=published_at]').fill(new Date(Date.now() - 60_000).toISOString());
    await form.locator('button[value=save]').click();
    expect((await page.goto(publicPage)).status()).toBe(200);
    await expect(page.locator('main h1')).toHaveText(title);

    expect((await page.goto(edit)).status()).toBe(200);
    await form.locator('button[value=delete]').click();
    expect((await page.goto(publicPage)).status()).toBe(404);
  } finally {
    await context.close();
  }
});

test('staff creates and edits a bilingual announcement', async ({browser}, info) => {
  test.setTimeout(300_000);
  const {context, page} = await isolatedAdmin(browser, info);
  try {
    expect((await page.goto('/admin/announcements/create')).status()).toBe(200);
    const title = `Ledger announcement ${randomUUID()}`;
    await submit(page, 'main form', {
      title,
      body: 'Local editorial announcement.',
      title_pl: 'Komunikat redakcyjny',
      body_pl: 'Lokalny komunikat redakcyjny.',
    });
    await expect(page).toHaveURL(/\/admin\/announcements\/[a-f0-9-]+\/edit$/);
    await expect(page.locator('textarea[name=title]')).toHaveValue(title);
    await submit(page, 'main form', {body: 'Updated local editorial announcement.'});
    expect((await page.reload()).status()).toBe(200);
    await expect(page.locator('textarea[name=body]')).toHaveValue('Updated local editorial announcement.');
  } finally {
    await context.close();
  }
});

test('staff returns an article from review to draft', async ({browser}, info) => {
  test.setTimeout(300_000);
  const {context, page} = await isolatedAdmin(browser, info);
  try {
    const slug = `ledger-guide-${randomUUID()}`;
    expect((await page.goto('/admin/wiki/articles/create')).status()).toBe(200);
    await submit(page, 'main form', {
      en_title: 'Ledger guide', en_slug: slug,
      en_summary: 'A local editorial guide.', en_markdown: 'Guide body.',
      pl_title: 'Poradnik redakcyjny', pl_slug: slug,
      pl_summary: 'Lokalny poradnik.', pl_markdown: 'Treść poradnika.',
      note: 'Local coverage qualification',
    });
    await expect(page).toHaveURL(/\/admin\/wiki\/articles\/[a-f0-9-]+\/edit$/);
    await submit(page, 'main form[action$="/submit-review"]');
    await submit(page, 'main form[action$="/return-draft"]');
    await expect(page.locator('main form[action$="/submit-review"]')).toBeVisible();
    expect((await page.request.get(`/en/wiki/articles/${slug}`)).status()).toBe(404);
  } finally {
    await context.close();
  }
});

test('staff changes a custom role permission through the roles form', async ({browser}, info) => {
  test.setTimeout(300_000);
  const {context, page} = await isolatedAdmin(browser, info);
  try {
    const id = await role(page);
    await action(page, 'form:has(button[value=set-permissions])', {
      role: id, permissions: 'admin.access',
    }, 'set-permissions');
    const rows = await query('SELECT permission FROM role_permissions WHERE role_id=$1', [id]);
    expect(rows).toEqual([{permission: 'admin.access'}]);
  } finally {
    await context.close();
  }
});

test('staff uses the editorial PUT endpoints to edit news and publish an event', async ({browser}, info) => {
  test.setTimeout(300_000);
  const {context, page} = await isolatedAdmin(browser, info);
  try {
    const slug = `ledger-${randomUUID()}`;
    const published = new Date(Date.now() - 60_000).toISOString();
    expect((await page.goto('/admin/news/create')).status()).toBe(200);
    await submit(page, 'main form', {
      slug, title: 'Ledger news', body: 'Original news text.', published_at: published,
      title_pl: 'Wiadomość redakcyjna', body_pl: 'Treść wiadomości.',
      translation_published_at: published,
    });
    await expect(page).toHaveURL(/\/admin\/news\/[a-f0-9-]+\/edit$/);
    const news = page.locator('main form:not([action$="/delete"])');
    const newsData = await news.evaluate(f => Object.fromEntries(new FormData(f)));
    const newsPath = await news.getAttribute('action');
    const headers = {origin: new URL(page.url()).origin};
    const updatedNews = await page.request.put(newsPath, {
      form: {...newsData, body: 'News updated with PUT.'}, headers, maxRedirects: 0,
    });
    expect(updatedNews.status()).toBe(302);
    expect((await page.goto(`/en/news/${slug}`)).status()).toBe(200);
    await expect(page.locator('main')).toContainText('News updated with PUT.');

    expect((await page.goto('/admin/events/create')).status()).toBe(200);
    await submit(page, 'main form', {
      starts_at: new Date(Date.now() - 3_600_000).toISOString().slice(0, 16),
      ends_at: new Date(Date.now() + 86_400_000).toISOString().slice(0, 16),
      en_title: 'Ledger event', en_slug: slug, en_summary: 'An editorial event.', en_body: 'Event body.',
      pl_title: 'Wydarzenie redakcyjne', pl_slug: slug, pl_summary: 'Lokalne wydarzenie.', pl_body: 'Treść wydarzenia.',
    });
    await expect(page).toHaveURL(/\/admin\/events\/[a-f0-9-]+\/edit$/);
    const edit = new URL(page.url()).pathname;
    const event = page.locator('main form:not([action$="/status"])');
    const eventData = await event.evaluate(f => Object.fromEntries(new FormData(f)));
    const eventPath = await event.getAttribute('action');
    const updatedEvent = await page.request.put(eventPath, {
      form: {...eventData, en_title: 'Updated ledger event'}, headers, maxRedirects: 0,
    });
    expect(updatedEvent.status()).toBe(303);
    expect((await page.goto(edit)).status()).toBe(200);
    const statusForm = page.locator('main form[action$="/status"]');
    const statusData = await statusForm.evaluate(f => Object.fromEntries(new FormData(f)));
    const statusPath = await statusForm.getAttribute('action');
    const status = await page.request.put(statusPath, {
      form: {...statusData, status: 'published'}, headers, maxRedirects: 0,
    });
    expect(status.status()).toBe(303);
    expect((await page.goto(`/en/events/${slug}`)).status()).toBe(200);
    await expect(page.locator('main h1')).toHaveText('Updated ledger event');
  } finally {
    await context.close();
  }
});
