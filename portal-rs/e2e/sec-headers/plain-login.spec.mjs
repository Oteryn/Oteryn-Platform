import { test, expect } from '@playwright/test';
import { cookie, expectSession, seedAccount } from '../security/auth-fixtures.mjs';
import { checkHeaders, checkIssued, sessionCookie, surface } from './policy.mjs';
import { matchedRouteHeader, ordinaryLogin, projectPath, publicRegistrations, publicRoute } from './registrations.mjs';
import { getFromPortal, loginForm, signIn } from './session-actions.mjs';

test('ordinary login GET and POST run with MFA disabled', async ({ page, context }, info) => {
  const locale = info.project.metadata.locale;
  const report = publicRegistrations();
  expect(report.mfaLoginRewrite).toBe(false);
  const paths = report.registrations.filter(row => row.get && ordinaryLogin(row.path) && projectPath(row.path, locale))
    .map(row => row.path).sort();
  const target = '/' + locale + '/login';
  expect(paths).toEqual([target, '/login'].sort());
  const account = await seedAccount(locale);
  const headers = { accept: 'text/html', 'accept-language': locale };
  for (const path of paths) {
    const response = await getFromPortal(page.request, path, { headers, maxRedirects: 0 });
    expect(response.status(), path).toBe(200);
    expect(response.headers()[matchedRouteHeader]).toBe(publicRoute(report, path).path);
    await checkHeaders(response, 'ordinary login ' + path, { csp: surface });
  }
  expect((await page.goto(target)).status()).toBe(200);
  await expect(page.locator(loginForm)).toBeVisible();
  const before = await cookie(context);
  const after = await signIn(page, account);
  expect(after.status()).toBe(302);
  expect(after.headers().location).toBe('/' + locale + '/');
  expect(after.headers()[matchedRouteHeader]).toBe(target);
  checkIssued(await checkHeaders(after, 'ordinary login redirect', { csp: surface }), sessionCookie);
  const signedIn = await cookie(context);
  expect(signedIn !== before, 'ordinary login rotates the session').toBe(true);
  await expectSession(signedIn, account.id);
});
