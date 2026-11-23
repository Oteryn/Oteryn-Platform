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
  } finally {
    await guest.close();
    await db.end();
  }
});
