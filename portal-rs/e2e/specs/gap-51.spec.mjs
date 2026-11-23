import {test, expect} from '../fixtures.mjs';
import {csrfRejected, submit} from '../helpers.mjs';
import {query, seededStaff, unique} from '../staff-rbac-helpers.mjs';
import {assertShell} from '../shell.mjs';

const operations = '/admin/world-operations';

test('scoped staff propose and independently approve a maintenance window', async ({page, browser}, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const reviewerContext = await browser.newContext({
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443',
    ignoreHTTPSErrors: true,
    viewport: info.project.use.viewport,
    locale: lang,
  });
  try {
    const reviewerPage = await reviewerContext.newPage();
    const requester = await seededStaff(page, lang, 'platform_admin');
    const reviewer = await seededStaff(reviewerPage, lang, 'platform_admin');
    const [{id: world}] = await query(
      'INSERT INTO world_status(id,name) VALUES (uuidv7(),$1) RETURNING id', [unique()],
    );
    const [{id: node}] = await query('SELECT uuidv7() AS id');
    // These are Platform planning scopes. The test supplies no Game evidence.
    await query(
      `INSERT INTO world_operation_planning_grants(account_id,world_id,action,can_review)
       VALUES ($1,$3,'restart',true),($2,$3,'restart',true)`,
      [requester, reviewer, world],
    );
    expect((await page.goto('/admin')).status()).toBe(200);
    await page.locator(`main nav a[href="${operations}"]`).click();
    await expect(page).toHaveURL(url => url.pathname === operations);
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    await page.locator('#scope').selectOption(`${world}/restart`);
    await page.locator('main form[method=get] button').click();

    const minute = offset => new Date(Date.now() + offset * 60_000).toISOString().slice(0, 16);
    const ticket = unique();
    const form = 'main form[method=post]';
    const payload = {
      starts: minute(60), ends: minute(120), ticket, target: node,
    };
    await submit(page, form, payload);
    await expect(page).toHaveURL(/\/admin\/world-operations\/[0-9a-f-]+$/);
    const path = new URL(page.url()).pathname;
    const id = path.split('/').at(-1);
    await expect(page.locator('[data-plan-ticket]')).toHaveText(ticket);
    await expect(page.locator('[data-plan-status]')).toHaveAttribute('data-plan-status', 'ops-pending');
    await expect(page.locator('main button[name=decision]')).toHaveCount(0);

    const response = await reviewerPage.goto(path);
    expect(response.status()).toBe(200);
    expect(response.headers()['cache-control']).toContain('no-store');
    await expect(reviewerPage.locator('[data-plan-ticket]')).toHaveText(ticket);
    await csrfRejected(reviewerPage, path, {decision: 'approve'});
    await reviewerPage.locator('main button[value=approve]').click();
    await expect(reviewerPage.locator('[data-plan-status]')).toHaveAttribute('data-plan-status', 'ops-approved');
    await expect(reviewerPage.locator('[data-game-status]')).toContainText(lang === 'pl' ? 'Niedostępne' : 'Unavailable');
    await assertShell(reviewerPage);
    expect(await query(
      'SELECT requester,reviewer,approved FROM world_operation_plans WHERE id=$1', [id],
    )).toEqual([{requester, reviewer, approved: true}]);
    expect(await query(
      `SELECT actor FROM audit_log WHERE action='admin.world_operations.reviewed' AND target=$1`, [id],
    )).toEqual([{actor: reviewer}]);

    await page.reload();
    await expect(page.locator('[data-plan-status]')).toHaveAttribute('data-plan-status', 'ops-approved');
  } finally {
    await reviewerContext.close();
  }
});
