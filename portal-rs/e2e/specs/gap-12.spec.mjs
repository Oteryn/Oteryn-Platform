import {randomUUID} from 'node:crypto';
import {test, expect} from '../fixtures.mjs';
import {action, query, seededAccount, seededLogin, seededMfaAccount, seededSudo} from '../staff-rbac-helpers.mjs';

const path = '/admin/console/sanctions';
const postForm = 'main form[method=post]';
const v7 = () => randomUUID().replace(/^(........-....-)4/, (_, prefix) => `${prefix}7`);

test('scoped staff review sanction apply and lift decisions', async ({page, browser}, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const list = `${path}?lang=${lang}`;
  const text = (pl, en) => lang === 'pl' ? pl : en;
  const requester = await seededMfaAccount(lang);
  const reviewer = await seededMfaAccount(lang);
  const account = await seededAccount(lang);
  const character = v7();
  await query(
    "INSERT INTO account_roles(account_id,role_id) SELECT a.id,r.id FROM accounts a CROSS JOIN roles r WHERE a.id=ANY($1::uuid[]) AND r.key='game_master_senior'",
    [[requester.id, reviewer.id]],
  );
  const [{id: world}] = await query("INSERT INTO world_status(id,name) VALUES (uuidv7(),'Review fixture') RETURNING id");
  const [{id: target}] = await query(
    'INSERT INTO admin_sanction_review_targets(world_id,account_id,character_id) VALUES ($1,$2,$3) RETURNING id',
    [world, account.id, character],
  );
  const [{id: report}] = await query(
    "INSERT INTO player_reports(reporter_account_id,request_key,report_type,category,target_reference) VALUES ($1,uuidv7(),'player','conduct',$2) RETURNING id",
    [account.id, character],
  );
  await query('INSERT INTO admin_sanction_review_grants(staff_id,target_id) VALUES ($1,$2)', [requester.id, target]);
  const reviewerContext = await browser.newContext({
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443',
    ignoreHTTPSErrors: true, viewport: info.project.use.viewport, locale: lang,
  });
  const other = await reviewerContext.newPage();
  const post = (actorPage, url, form) => actorPage.request.post(url, {
    form, headers: {origin: new URL(actorPage.url()).origin}, maxRedirects: 0,
  });
  try {
    await seededLogin(page, lang, requester);
    await seededSudo(page, lang, requester);
    expect((await page.goto(`/admin/console/players?lang=${lang}`)).status()).toBe(200);
    await page.getByRole('link', {name: text('Weryfikacja sankcji', 'Sanction reviews')}).click();
    await expect(page).toHaveURL(new RegExp(`${path}\\?lang=${lang}$`));
    await expect(page.getByText(text('Niedostępne', 'Unavailable'), {exact: true})).toBeVisible();
    const csrf = await page.locator(`${postForm} input[name=_csrf]`).inputValue();
    expect((await post(page, path, {
      _csrf: csrf, lang, target, report, kind: 'jail', reason: 'conduct', expires: '2000-01-01T01:00',
    })).status()).toBe(400);

    await seededLogin(other, lang, reviewer);
    await seededSudo(other, lang, reviewer);
    for (const kind of ['ban', 'mute', 'jail']) {
      await page.goto(list);
      const expires = new Date(Date.now() + 3_600_000).toISOString().slice(0, 16);
      await action(page, postForm, {target, report, kind, reason: 'conduct', expires});
      await expect(page).toHaveURL(new RegExp(`${path}\\?lang=${lang}&id=[0-9a-f-]{36}$`));
      const detail = new URL(page.url());
      const id = detail.searchParams.get('id');
      expect(id).toMatch(/^[0-9a-f-]{36}$/);
      await expect(page.getByText(text('Nałożenie oczekuje na weryfikację', 'Apply review pending'))).toBeVisible();
      await expect(page.locator(postForm)).toHaveCount(0);

      if (kind === 'ban') {
        expect((await other.goto(detail.href)).status()).toBe(403);
        await query('INSERT INTO admin_sanction_review_grants(staff_id,target_id) VALUES ($1,$2)', [reviewer.id, target]);
        expect((await post(page, `${path}/${id}`, {_csrf: csrf, lang, action: 'approve', revision: '1'})).status()).toBe(403);
      }
      expect((await other.goto(detail.href)).status()).toBe(200);
      await action(other, postForm);
      await expect(other.getByText(text('Zatwierdzono nałożenie na Platformie', 'Apply review approved')).first()).toBeVisible();
      await page.reload();
      await action(page, postForm, {reason: 'review'});
      await expect(page.getByText(text('Zniesienie oczekuje na weryfikację', 'Lift review pending')).first()).toBeVisible();
      await other.reload();
      await action(other, postForm);
      await expect(other.getByText(text('Zatwierdzono zniesienie na Platformie', 'Lift review approved')).first()).toBeVisible();
      await expect(other.getByText(text('Niedostępne', 'Unavailable'), {exact: true})).toBeVisible();
      expect(await other.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
      const decisions = await query(
        'SELECT actor, action FROM audit_log WHERE target=$1 ORDER BY (after->>\'revision\')::int',
        [id],
      );
      expect(decisions).toEqual([
        {actor: requester.id, action: 'sanctions.apply_review_requested'},
        {actor: reviewer.id, action: 'sanctions.apply_review_approved'},
        {actor: requester.id, action: 'sanctions.lift_review_requested'},
        {actor: reviewer.id, action: 'sanctions.lift_review_approved'},
      ]);
    }
    expect(await query("SELECT request_id FROM admin_control_requests WHERE actor=ANY($1::uuid[]) AND operation IN ('staff.ban','staff.unban')", [[requester.id, reviewer.id]])).toEqual([]);
    expect(await query("SELECT operation_id FROM support_enforcement_commands WHERE account_id=$1", [account.id])).toEqual([]);
  } finally {
    await reviewerContext.close();
  }
});
