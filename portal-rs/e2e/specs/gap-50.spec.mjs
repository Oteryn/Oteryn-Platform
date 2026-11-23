import { test, expect } from '../fixtures.mjs';
import { readFileSync } from 'node:fs';
import {
  action, query, seededAccount, seededLogin, seededStaff,
} from '../staff-rbac-helpers.mjs';

const queue = '/admin/character-cases';

test('i18n-parity for staff character cases', () => {
  const template = readFileSync(new URL('../../templates/admin/character_cases.html', import.meta.url), 'utf8');
  const templateKeys = [...template.matchAll(/self\.label\("([^"]+)"\)/g)].map(([, key]) => key);
  const keys = lang => {
    const source = readFileSync(new URL(`../../templates/admin/${lang}.ftl`, import.meta.url), 'utf8');
    const messages = [...source.matchAll(/^character-case-([\w-]+)\s*=\s*(\S.*)$/gm)]
      .map(([, key]) => key);
    expect(new Set(messages).size).toBe(messages.length);
    expect(messages).toEqual(expect.arrayContaining([
      ...templateKeys, 'limited', 'invalid', 'denied', 'missing', 'conflict', 'storage',
      'create', 'rename', 'schedule', 'restore', 'repair', 'pending', 'accept', 'return',
      'Female', 'Male', 'Knight', 'Paladin', 'Sorcerer', 'Druid', 'Monk',
    ]));
    return messages.sort();
  };
  expect(keys('en')).toEqual(keys('pl'));
});

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
    await requester.locator('main form[method=get] button').click();
    await expect(requester.locator('main input[name=ticket]')).toHaveValue(ticket);
    await action(requester, 'main form[method=post]', {
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
    await expect(requester.locator('main form[method=post]')).toHaveCount(0);

    await reviewer.goto(queue);
    const casePath = new URL(caseUrl).pathname + new URL(caseUrl).search;
    await reviewer.locator(`a[href="${casePath}"]`).click();
    const note = 'Ready for Character Authority qualification.\nEvidence checked against the ticket.';
    await action(reviewer, 'main form[method=post]', { note }, 'accept');
    await expect(reviewer).toHaveURL(caseUrl);
    await expect(reviewer.locator('main')).toContainText(
      lang === 'pl' ? 'Przyjęto do kwalifikacji' : 'Accepted for qualification',
    );
    const status = reviewer.locator('section[aria-labelledby=game-status]');
    await expect(status.locator('dd')).toHaveText(
      Array(3).fill(lang === 'pl' ? 'Niedostępne' : 'Unavailable'),
    );
    await expect(reviewer.locator('main form[method=post]')).toHaveCount(0);
    await expect(reviewer.locator('dd').filter({ hasText: 'Evidence checked against the ticket.' }))
      .toHaveJSProperty('innerText', note);
    expect(await reviewer.evaluate(() => document.documentElement.scrollWidth))
      .toBeLessThanOrEqual(info.project.use.viewport.width);
    expect((await page.request.get(casePath)).status()).toBe(403);

    await query("UPDATE tickets SET status='closed', closed_at=now() WHERE id=$1", [ticket]);
    const filteredQueue = `${queue}?ticket=${ticket}`;
    expect((await requester.goto(filteredQueue)).status()).toBe(200);
    await expect(requester.locator('main form[method=get]')).toHaveCount(0);
    await expect(requester.locator(`a[href="${casePath}"]`)).toBeVisible();
    expect((await requester.request.get(`${filteredQueue}&operation=repair`)).status()).toBe(409);
    expect((await requester.request.get(`${queue}?operation=unknown`)).status()).toBe(400);
  } finally {
    await Promise.all([requesterContext.close(), reviewerContext.close()]);
  }
});
