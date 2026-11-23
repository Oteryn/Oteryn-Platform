import { readFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { login, pool, register, submit } from '../helpers.mjs';

test('account navigation leads to referral acceptance and one recorded starter entitlement', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const accountPath = `/${lang}/account/referrals`;
  const db = pool();
  const guest = await browser.newContext({
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443',
    ignoreHTTPSErrors: true,
    viewport: page.viewportSize(),
    locale: lang,
  });

  try {
    const owner = await register(page, lang);
    await login(page, lang, owner);
    await page.goto('/account');
    if (!(await page.locator('#portal-navigation').isVisible())) {
      const fallbackMenu = page.locator('.om-fallback-menu > summary');
      await (await fallbackMenu.isVisible() ? fallbackMenu : page.locator('#portal-menu-toggle')).click();
    }
    const accountMenu = page.locator('.om-fallback-account');
    if (await accountMenu.isVisible()) {
      await accountMenu.locator('summary').click();
      await accountMenu.locator('a[href$="/account/referrals"]').click();
    } else {
      await page.locator('[data-account-link]').click();
      await page.locator('#podglad-konta a[href$="/account/referrals"]').click();
    }
    await expect(page).toHaveURL(new RegExp(`${accountPath}$`));
    await page.locator('button[name="kind"][value="link"]').click();
    const invitation = await page.locator('#invite-link').inputValue();

    await page.goto(invitation);
    await submit(page);
    await expect(page.locator('main [role="status"]')).toHaveText(
      lang === 'pl' ? 'Nie można przyjąć tego zaproszenia.' : 'This invitation cannot be accepted.',
    );
    expect((await page.goto(`/${lang}/referrals/malformed`)).status()).toBe(400);
    const expired = await db.query(
      `INSERT INTO referral_email_invitations(inviter_id, recipient, kind, created_at)
       SELECT id, $2, 'new', now() - interval '15 days' FROM accounts WHERE email=$1
       RETURNING id`, [owner.email, `expired-${owner.email}`],
    );
    expect((await page.goto(`/${lang}/referrals/${expired.rows[0].id}`)).status()).toBe(403);

    const inviteePage = await guest.newPage();
    await inviteePage.goto(invitation);
    await submit(inviteePage);
    await expect(inviteePage).toHaveURL(new RegExp(`/${lang}/register$`));
    const invitee = await register(inviteePage, lang);
    await login(inviteePage, lang, invitee);
    await inviteePage.goto(accountPath);
    await expect(inviteePage.locator('#referral-attribution')).toHaveText(
      lang === 'pl' ? 'Twoje zaproszenie z polecenia zostało przyjęte.' : 'Your referral invitation was accepted.',
    );
    await expect(inviteePage.locator('[data-reward-state="game-unavailable"]')).toHaveCount(1);
    expect(await inviteePage.evaluate(() =>
      document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);

    await inviteePage.goto(invitation);
    await submit(inviteePage);
    await expect(inviteePage).toHaveURL(new RegExp(`${accountPath}$`));
    await expect(inviteePage.locator('[data-reward-state="game-unavailable"]')).toHaveCount(1);
    const result = await db.query(
      `SELECT o.state, o.run_after = 'infinity' AS held
       FROM referral_campaign_invitations i
       JOIN accounts a ON a.id=i.invitee_id
       JOIN referral_campaign_rewards r ON r.operation_key='starter:'||i.id
       JOIN outbox o ON o.kind='referral.entitlement' AND o.idempotency_key=r.operation_key
       WHERE a.email=$1`, [invitee.email],
    );
    expect(result.rows).toEqual([{ state: 'pending', held: true }]);
    expect((await inviteePage.goto(`/admin/referrals?lang=${lang}`)).status()).toBe(403);

    await page.goto(accountPath);
    await expect(page.locator('section[aria-labelledby="invitations"] li')).toHaveText(
      lang === 'pl' ? /Oczekiwanie na płatny zakup/ : /Waiting for a paid purchase/,
    );

    const older = lang === 'pl' ? 'Starsze wpisy' : 'Older entries';
    const latest = lang === 'pl' ? 'Najnowsze wpisy' : 'Latest entries';
    await db.query(
      `WITH invitees AS (
         INSERT INTO accounts(id, email, password_hash, status, locale)
         SELECT uuidv7(), uuidv7()||'@local.test', 'non-loginable-fixture', 'active', $2
         FROM generate_series(1, 100) RETURNING id
       )
       INSERT INTO referral_campaign_invitations(inviter_id, invitee_id, kind, first_paid_at, created_at)
       SELECT a.id, invitees.id, 'new', now(), now() + interval '1 second'
       FROM accounts a CROSS JOIN invitees WHERE a.email=$1`, [owner.email, lang],
    );
    await page.goto(accountPath);
    const invitations = page.locator('section[aria-labelledby="invitations"]');
    await expect(invitations.locator('li')).toHaveCount(100);
    await invitations.getByRole('link', { name: older }).click();
    await expect(invitations.locator('li')).toHaveText(
      lang === 'pl' ? /Oczekiwanie na płatny zakup/ : /Waiting for a paid purchase/,
    );
    await invitations.getByRole('link', { name: latest }).click();
    await expect(invitations.locator('li')).toHaveCount(100);

    await db.query(
      `INSERT INTO referral_campaign_rewards(operation_key, account_id, kind, amount, applied_at, created_at)
       SELECT 'browser-history:'||a.id||':'||n, a.id, 'premium', 1, now(), now() + interval '1 second'
       FROM accounts a CROSS JOIN generate_series(1, 100) n WHERE a.email=$1`, [invitee.email],
    );
    await inviteePage.goto(accountPath);
    const rewards = inviteePage.locator('section[aria-labelledby="rewards"]');
    await expect(rewards.locator('li[data-reward-state="applied"]')).toHaveCount(100);
    await expect(rewards.locator('li[data-reward-state="game-unavailable"]')).toHaveCount(0);
    await rewards.getByRole('link', { name: older }).click();
    await expect(rewards.locator('li')).toHaveCount(1);
    await expect(rewards.locator('li[data-reward-state="game-unavailable"]')).toHaveCount(1);
    await rewards.getByRole('link', { name: latest }).click();
    await expect(rewards.locator('li[data-reward-state="applied"]')).toHaveCount(100);
    expect((await inviteePage.goto(`${accountPath}?rewards_before=malformed`)).status()).toBe(400);
    await expect(inviteePage.locator('main [role="status"]')).toHaveText(
      lang === 'pl' ? 'Link do historii jest nieprawidłowy.' : 'This history link is invalid.',
    );
  } finally {
    await guest.close();
    await db.end();
  }
});

test('i18n-parity for referrals', () => {
  const keys = (lang) => [...readFileSync(
    new URL(`../../crates/community/src/referrals/${lang}.ftl`, import.meta.url), 'utf8',
  ).matchAll(/^([\w-]+)\s*=\s*\S/gm)].map((match) => match[1]).sort();
  const pl = keys('pl');
  expect(pl).toContain('operations-title');
  expect(pl).toContain('older');
  expect(new Set(pl).size).toBe(pl.length);
  expect(keys('en')).toEqual(pl);
});
