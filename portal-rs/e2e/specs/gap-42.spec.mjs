import {test, expect} from '../fixtures.mjs';
import {csrfRejected, login, pool} from '../helpers.mjs';
import {paidTopup, staffContext, uuid7} from '../staff-commerce-helpers.mjs';
import {seededAccount} from '../staff-rbac-helpers.mjs';

test('player transfer replay has one history record and staff can look it up', async ({page, browser}, info) => {
  const lang = info.project.metadata.locale;
  const en = lang === 'en';
  const {id: recipientId} = await seededAccount(lang);
  const sender = await seededAccount(lang);
  await login(page, lang, sender);
  const db = pool();
  const staff = await staffContext(browser, info);
  try {
    const funded = await paidTopup(page, lang, db);
    expect((await page.goto(`/${lang}/shop`)).status()).toBe(200);
    await expect(page.getByRole('status')).toHaveText(en
      ? 'This operation is unavailable. No success has been confirmed.'
      : 'Operacja jest niedostępna. Powodzenie nie zostało potwierdzone.');
    await page.locator(`a[href="/${lang}/shop/transfer"]`).click();
    const form = page.locator('main form');
    const fields = {
      _csrf: await form.locator('[name=_csrf]').inputValue(),
      operation: await form.locator('[name=operation]').inputValue(),
      recipient: recipientId,
      amount: '7',
    };
    const transfer = `/${lang}/shop/transfer`;
    await csrfRejected(page, transfer, fields);
    const post = (amount) => page.request.post(transfer, {
      form: {...fields, amount},
      headers: {origin: new URL(page.url()).origin},
      maxRedirects: 0,
    });
    expect((await post('-7')).status()).toBe(422);
    expect((await post(String(Number(funded.coins) + 1))).status()).toBe(403);
    await form.locator('[name=recipient]').fill(recipientId);
    await form.locator('[name=amount]').fill(fields.amount);
    await form.getByRole('button').click();
    await expect(page).toHaveURL(new RegExp(`/${lang}/shop/history$`));
    expect((await post(fields.amount)).status()).toBe(303);
    expect((await post('8')).status()).toBe(409);
    await page.reload();
    const entry = page.locator(`[data-wallet-operation="${fields.operation}"]`);
    await expect(entry).toHaveCount(1);
    await expect(entry.getByRole('heading')).toHaveText(en ? 'Coin transfer' : 'Przelew coinów');
    await expect(page.locator('main dl dd').first()).toHaveText(String(Number(funded.coins) - 7));
    const credits = await db.query(
      'SELECT available_delta FROM wallet_entries WHERE operation_id = $1 AND account_id = $2',
      [fields.operation, recipientId],
    );
    expect(credits.rows.map(({available_delta}) => Number(available_delta))).toEqual([7]);

    const admin = await staff.newPage();
    await admin.setViewportSize(page.viewportSize());
    await admin.setExtraHTTPHeaders({'accept-language': lang});
    expect((await admin.goto('/admin/commerce')).status()).toBe(200);
    await admin.locator('a[href="/admin/commerce/operation-status"]').click();
    const lookup = admin.locator('main form');
    await lookup.locator('[name=source]').selectOption('wallet');
    await lookup.locator('[name=id]').fill(fields.operation);
    await lookup.getByRole('button').click();
    await expect(admin.getByRole('heading', {level: 2})).toHaveText(en ? 'Platform record' : 'Zapis platformy');
    await expect(admin.locator('main dl')).toContainText(recipientId);
    await expect(admin.locator('main dl')).toContainText(en ? 'Coin operation committed' : 'Zatwierdzono operację na coinach');
    expect(await admin.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);

    await lookup.locator('[name=id]').fill(uuid7());
    await lookup.getByRole('button').click();
    await expect(admin.getByRole('status')).toContainText(en ? 'may still be in flight' : 'może nadal trwać');
  } finally {
    await staff.close();
    await db.end();
  }
});
