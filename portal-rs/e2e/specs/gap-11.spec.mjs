import { test, expect } from '../fixtures.mjs';
import fs from 'node:fs';
import { csrfRejected, mailLink, submit } from '../helpers.mjs';
import { action, audit, query, seededAccount, seededLogin, seededStaff, unique } from '../staff-rbac-helpers.mjs';

test('staff find a player and correct account details', async ({ page, browser }, info) => {
  test.setTimeout(120_000);
  const lang = info.project.metadata.locale;
  const account = await seededAccount(lang);
  await seededLogin(page, lang, account);
  const context = await browser.newContext({
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443', ignoreHTTPSErrors: true,
    locale: lang, viewport: info.project.use.viewport,
  });
  try {
    const admin = await context.newPage();
    await seededStaff(admin, lang, 'platform_admin');
    await admin.goto('/admin');
    await admin.locator('nav a[href="/admin/players"]').click();

    const detail = `/admin/players/${account.id}`;
    const result = () => admin.locator(`a[href="${detail}"]`);
    async function search(field, match, term) {
      await admin.locator('[name=field]').selectOption(field);
      await admin.locator('[name=match]').selectOption(match);
      await admin.locator('[name=q]').fill(term);
      await admin.locator('form[method=get] button').click();
      await expect(result()).toHaveCount(1);
    }

    await search('email', 'exact', account.email.toUpperCase());
    await search('email', 'prefix', account.email.slice(0, -5));
    await search('account-id', 'prefix', account.id.slice(0, 28));
    await search('account-id', 'exact', account.id);

    const [{ character_id, world_id }] = await query(
      "SELECT uuidv7() AS character_id, id AS world_id FROM world_status WHERE name='Local Bazaar World'",
    );
    const publication = await fetch(`${process.env.PORTAL_E2E_GAME_CLOSURE_CONTROL_URL}/portfolio`, {
      method: 'POST', headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ account_id: account.id, character_id, world_id }),
    });
    expect(publication.status).toBe(200);
    const { name } = await publication.json();
    await search('character', 'exact', name.toUpperCase());
    await search('character', 'prefix', name.slice(0, -3));

    await result().click();
    await action(admin, 'form:has(button[value=edit-public-status-visible])',
      { value: 'true', reason: unique() }, 'edit-public-status-visible');
    await expect(admin.locator('#edit-public-status-visible')).toHaveValue('true');
    const displayName = 'Alina Żółć <pomoc>';
    const nameReason = unique();
    await action(admin, 'form:has(button[value=edit-display-name])',
      { value: displayName, reason: nameReason }, 'edit-display-name');
    await expect(admin.locator('#edit-display-name')).toHaveValue(displayName);
    await expect(admin.locator('pomoc')).toHaveCount(0);
    expect(await admin.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);

    const email = `${unique()}@example.invalid`;
    const emailReason = unique();
    await csrfRejected(admin, detail, { action: 'edit-email', value: email, reason: emailReason });
    await action(admin, 'form:has(button[value=edit-email])',
      { value: email, reason: emailReason }, 'edit-email');
    await expect(admin.locator('#edit-email')).toHaveValue(email);
    await page.goto(`/${lang}/account/sessions`);
    await expect(page).toHaveURL(/\/login(?:\?|$)/);

    expect(await query('SELECT email, display_name, status, email_verified_at FROM accounts WHERE id=$1', [account.id]))
      .toEqual([{ email, display_name: displayName, status: 'pending_verification', email_verified_at: null }]);
    expect((await query("SELECT after->>'field' AS field, after->>'reason' AS reason FROM audit_log WHERE target=$1 AND action='players.account.edited'", [account.id])))
      .toEqual(expect.arrayContaining([{ field: 'display-name', reason: nameReason }, { field: 'email', reason: emailReason }]));
    await audit(admin, 'players.account.edited', account.id);

    const link = await mailLink(page, email, /https:\/\/localhost:18443\/(?:pl|en)\/verify-email\?token=[a-f0-9]+/);
    expect((await page.goto(link)).status()).toBe(200);
    await submit(page, 'main form');
    await seededLogin(page, lang, { ...account, email });
    expect((await page.goto(`/${lang}/account/sessions`)).status()).toBe(200);
    await expect(page).toHaveURL(new RegExp(`/${lang}/account/sessions$`));
  } finally {
    await context.close();
  }
});

test('i18n-parity for player search and account editing', () => {
  for (const dir of ['../../crates/admin/src/players/', '../../templates/admin/']) {
    const keys = lang => Array.from(
      fs.readFileSync(new URL(`${dir}${lang}.ftl`, import.meta.url), 'utf8')
        .matchAll(/^([a-zA-Z][\w-]*)\s*=/gm), match => match[1],
    ).sort();
    const english = keys('en');
    expect(english.length).toBeGreaterThan(0);
    expect(new Set(english).size).toBe(english.length);
    expect(keys('pl')).toEqual(english);
  }
});
