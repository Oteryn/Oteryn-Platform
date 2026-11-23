import {test, expect} from '../fixtures.mjs';
import {register, login, submit, pool} from '../helpers.mjs';
import * as OTPAuth from 'otpauth';

test('staff reviews character remediation qualification from Players', async ({page}, info) => {
 const lang = info.project.metadata.locale;
 const players = '/admin/console/players';
 const overview = `${players}?lang=${lang}&remediation=overview`;
 const staff = await register(page, lang);
 const db = pool();
 try {
  await db.query(
   "INSERT INTO account_roles(account_id,role_id) SELECT a.id,r.id FROM accounts a CROSS JOIN roles r WHERE a.email=$1 AND r.key='platform_admin'",
   [staff.email],
  );
 } finally {
  await db.end();
 }
 await login(page, lang, staff);
 expect((await page.goto(overview)).status()).toBe(403);
 await page.goto(`/${lang}/mfa`);
 await submit(page, 'main form[action$="/mfa/enroll"]', {password: staff.password});
 const secret = await page.locator('main .op-prose').innerText();
 const totp = new OTPAuth.TOTP({secret, algorithm: 'SHA1', digits: 6, period: 30});
 await submit(page, 'main form[action$="/mfa/confirm"]', {password: staff.password, code: totp.generate()});
 const code = (await page.locator('main textarea[readonly]').allTextContents())[0];
 await login(page, lang, staff);
 await submit(page, 'main form[action$="/mfa/challenge"]', {code});

 expect((await page.goto(`${players}?lang=${lang}`)).status()).toBe(200);
 await page.locator(`nav a[href="${overview}"]`).click();
 await expect(page).toHaveURL(overview);
 await expect(page.locator('main [data-state="unavailable"]')).toHaveCount(5);
 await page.locator('main a[href$="remediation=confiscate"]').click();
 await expect(page.locator('main')).toContainText('character.item.confiscate');
 await expect(page.locator('main')).toContainText('R4');
 await expect(page.locator('main')).toContainText(lang === 'pl' ? 'Niedostępne' : 'Unavailable');
 await expect(page.locator('main form[method="post"]')).toHaveCount(0);
 expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
  await page.evaluate(() => document.documentElement.clientWidth),
 );
 expect((await page.request.get(`${players}?remediation=unknown`)).status()).toBe(400);
});
