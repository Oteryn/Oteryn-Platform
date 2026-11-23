import { test, expect } from '../fixtures.mjs';
import {
  action, query, seededAccount, seededLogin, seededStaff,
} from '../staff-rbac-helpers.mjs';

const queue = '/admin/character-cases';

test('staff files a character repair case and an independent reviewer records the Platform decision', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const options = {
    baseURL: info.project.use.baseURL,
    ignoreHTTPSErrors: true,
    locale: lang,
    viewport: info.project.use.viewport,
  };
  const requesterContext = await browser.newContext(options);
  const reviewerContext = await browser.newContext(options);
  try {
    const player = await seededAccount(lang);
    const destination = await seededAccount(lang);
    await seededLogin(page, lang, player);
    await page.goto(`/${lang}/support/tickets/create`);
    await action(page, 'main form', {
      category: 'character',
      subject: 'Character ownership review',
      body: 'Please review the ownership history for this character.',
    });
    await expect(page).toHaveURL(/support\/tickets\/[a-f0-9-]+$/);
    const ticket = new URL(page.url()).pathname.split('/').at(-1);

    const requester = await requesterContext.newPage();
    const reviewer = await reviewerContext.newPage();
    await seededStaff(requester, lang, 'support');
    const reviewerId = await seededStaff(reviewer, lang, 'support');
    await query(
      `WITH role AS (
         INSERT INTO roles(key) VALUES ($2) RETURNING id
       ), permissions AS (
         INSERT INTO role_permissions(role_id,permission)
         SELECT id,'support.character-cases.review' FROM role
       )
       INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM role`,
      [reviewerId, `case_review_${reviewerId.replaceAll("-", "")}`],
    );
    const [{ world, character }] = await query('SELECT uuidv7() AS world, uuidv7() AS character');

    await requester.goto('/admin');
    await expect(requester.locator(`main nav a[href="${queue}"]`)).toBeVisible();
    await requester.goto(`/admin/support/tickets/${ticket}`);
    await requester.locator(`a[href="${queue}?ticket=${ticket}"]`).click();
    await requester.locator('select[name=operation]').selectOption('repair');
    await requester.locator('form[method=get] button').click();
    await expect(requester.locator('input[name=ticket]')).toHaveValue(ticket);
    await action(requester, 'form[method=post]', {
      reviewer: reviewerId,
      world,
      character,
      expected_revision: '42',
      destination: destination.id,
      evidence: 'Ticket message <ownership discrepancy>',
    });
    await expect(requester).toHaveURL(/\/admin\/character-cases\?id=[a-f0-9-]+$/);
    const caseUrl = requester.url();
    await expect(requester.locator('main')).toContainText(
      lang === 'pl' ? 'Oczekuje na niezależną ocenę' : 'Awaiting independent review',
    );
    await expect(requester.locator('main')).toContainText('Ticket message <ownership discrepancy>');
    await expect(requester.locator('form[method=post]')).toHaveCount(0);

    await reviewer.goto(queue);
    const casePath = new URL(caseUrl).pathname + new URL(caseUrl).search;
    await reviewer.locator(`a[href="${casePath}"]`).click();
    await action(reviewer, 'form[method=post]', { note: 'Ready for Character Authority qualification.' }, 'accept');
    await expect(reviewer).toHaveURL(caseUrl);
    await expect(reviewer.locator('main')).toContainText(
      lang === 'pl' ? 'Przyjęto do kwalifikacji' : 'Accepted for qualification',
    );
    const status = reviewer.locator('section[aria-labelledby=game-status]');
    await expect(status.locator('dd')).toHaveText(
      Array(3).fill(lang === 'pl' ? 'Niedostępne' : 'Unavailable'),
    );
    await expect(reviewer.locator('form[method=post]')).toHaveCount(0);
    expect(await reviewer.evaluate(() => document.documentElement.scrollWidth))
      .toBeLessThanOrEqual(info.project.use.viewport.width);
    expect((await page.request.get(casePath)).status()).toBe(403);
  } finally {
    await Promise.all([requesterContext.close(), reviewerContext.close()]);
  }
});
