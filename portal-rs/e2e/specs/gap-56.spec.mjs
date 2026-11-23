import { randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { submit } from '../helpers.mjs';
import { assertShell } from '../shell.mjs';
import { action, seededAccount, seededLogin, seededStaff } from '../staff-rbac-helpers.mjs';

test('article replies go from reader submission through review and removal', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const origin = new URL(info.project.use.baseURL).origin;
  const viewport = info.project.use.viewport;
  const staffContext = await browser.newContext({ baseURL: origin, ignoreHTTPSErrors: true, locale: lang, viewport });
  const readerContext = await browser.newContext({ baseURL: origin, ignoreHTTPSErrors: true, locale: lang, viewport });
  try {
    const editor = await staffContext.newPage();
    await seededStaff(editor, lang, 'content_editor');
    const slug = `discussion-${randomUUID()}`;
    const published = new Date(Date.now() - 60_000).toISOString();
    const article = `/${lang}/news/${slug}`;
    const discussion = `${article}/discussion`;
    await editor.goto('/admin/news/create');
    await submit(editor, 'main form', {
      slug, title: 'Discussion article', body: 'Article to discuss.', published_at: published,
      title_pl: 'Artykuł do dyskusji', body_pl: 'Artykuł do omówienia.', translation_published_at: published,
    });
    await expect(editor).toHaveURL(/\/admin\/news\/[a-f0-9-]+\/edit$/);

    await seededLogin(page, lang, await seededAccount(lang));
    await page.goto(article);
    await page.locator(`main a[href="${discussion}"]`).click();
    await expect(page).toHaveURL(`${origin}${discussion}`);
    await assertShell(page);
    const compose = `main form[action="${discussion}"]`;
    const token = await page.locator(`${compose} input[name="_csrf"]`).inputValue();
    const headers = { origin };
    const reply = `Reply ${slug} observes that 2 < 3.`;
    const invalid = await page.request.post(discussion, {
      headers, form: { _csrf: token, body: '**Unsupported formatting**' },
    });
    expect(invalid.status()).toBe(422);
    await submit(page, compose, { body: reply });
    const ownReply = page.locator('main article').filter({ hasText: reply });
    await expect(ownReply).toContainText(lang === 'pl' ? 'Oczekuje na moderację' : 'Awaiting review');
    const id = (await ownReply.getAttribute('id')).slice('reply-'.length);

    const visitor = await readerContext.newPage();
    await visitor.goto(discussion);
    await expect(visitor.locator('main')).not.toContainText(reply);

    await editor.goto(`/admin/news?lang=${lang}`);
    await editor.locator('main a[href^="/admin/news/discussions?"]').click();
    await editor.locator(`main a[href="/admin/news/discussions/${id}?lang=${lang}"]`).click();
    const review = 'main form:has(select[name=decision])';
    await assertShell(editor);
    await action(editor, review, { decision: 'publish', reason: 'Suitable for the article.' });
    await expect(editor.locator('main')).toContainText('Suitable for the article.');

    await visitor.reload();
    await expect(visitor.locator('main')).toContainText(reply);
    await expect(visitor.locator('main')).not.toContainText('Suitable for the article.');
    await assertShell(visitor);
    await seededLogin(visitor, lang, await seededAccount(lang));
    await visitor.goto(discussion);
    const visitorToken = await visitor.locator(`${compose} input[name="_csrf"]`).inputValue();
    const edit = `${discussion}/${id}/edit`;
    expect((await visitor.request.post(edit, {
      headers, form: { _csrf: visitorToken, body: 'Different text', revision: '2' },
    })).status()).toBe(404);
    expect((await visitor.request.post(`${discussion}/not-a-uuid/edit`, {
      headers, form: { _csrf: visitorToken, body: 'Different text', revision: '2' },
    })).status()).toBe(400);

    await action(editor, review, { decision: 'hide', reason: 'Removed after a second review.' });
    await expect(editor.locator('main')).toContainText('Removed after a second review.');
    await expect(editor.locator('main')).toContainText('Suitable for the article.');
    await visitor.reload();
    await expect(visitor.locator('main')).not.toContainText(reply);
    await page.reload();
    await expect(ownReply).toContainText(lang === 'pl' ? 'Ukryta' : 'Hidden');
    await expect(ownReply.locator('form')).toHaveCount(0);
    for (const screen of [page, editor, visitor]) {
      expect(await screen.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(viewport.width);
    }
  } finally {
    await Promise.all([staffContext.close(), readerContext.close()]);
  }
});
