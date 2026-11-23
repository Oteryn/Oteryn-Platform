import { atPath, expect, localized, password, submit, test, visit } from './fixtures.mjs';
import { verifiedLogin } from './identity.mjs';

const corePages = [
  ['account', 'Przegląd konta', 'Account overview'],
  ['account/privacy', 'Prywatność i język', 'Privacy and language'],
  ['account/sessions', 'Sesje web', 'Web sessions'],
  ['account/passkeys', 'Klucze dostępu', 'Passkeys'],
  ['account/security', 'Bezpieczeństwo konta', 'Account security'],
  ['account/security/email', 'Bezpieczeństwo adresu e-mail', 'Email security'],
  ['account/sudo', 'Potwierdź hasło', 'Confirm your password'],
  ['password/change', 'Zmień hasło', 'Change password'],
  ['mfa', 'Bezpieczeństwo konta', 'Account security'],
  ['account/notifications', 'Preferencje powiadomień', 'Notification preferences'],
  ['panel/premium', 'Premium', 'Premium'],
  ['panel/payments', 'Płatności', 'Payments'],
  ['support/tickets', 'Zgłoszenia pomocy', 'Support tickets'],
  ['support/reports', 'Zgłoszenia', 'Reports'],
];

test('5.1 Account pages load with localized content', { tag: '@mail' }, async ({ mail, page, lang, account }) => {
  await verifiedLogin(page, lang, account, mail);
  for (const [path, pl, en] of corePages) {
    const route = `/${lang}/${path}`;
    await test.step(`GET ${route}`, async () => {
      await visit(page, route);
      await localized(page, lang, lang === 'pl' ? pl : en);
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
      if (path === 'panel/payments') {
        await expect(page.locator('main')).toContainText(lang === 'pl' ? 'Brak płatności.' : 'No payments yet.');
      }
    });
  }

  const sudo = `/${lang}/account/sudo`;
  await visit(page, sudo);
  await submit(page, sudo, 302, { current_password: account.password });
  await atPath(page, `/${lang}/account/security`);
  await visit(page, `/${lang}/account/privacy/export`);
  await localized(page, lang, lang === 'pl' ? 'Pobierz dane konta' : 'Download account data');
});

test('5.2 Account mutation requires form verification', { tag: '@mail' }, async ({ mail, page, lang, account }) => {
  await verifiedLogin(page, lang, account, mail);
  const route = `/${lang}/password/change`;
  await visit(page, route);
  await expect(page.locator('main form input[name="_csrf"]')).toHaveCount(1);
  const next = password();
  await page.locator('main form input[name="_csrf"]').evaluate(input => input.remove());
  await submit(page, route, 403, {
    current_password: account.password, password: next, password_confirmation: next,
  });
});

test('5.3 A visitor without a session is redirected from an account page', async ({ page, lang }) => {
  const route = `/${lang}/account`;
  const response = await visit(page, route, 200, { destination: `/${lang}/login` });
  const request = response.request().redirectedFrom();
  expect((await request?.response())?.status(), `GET ${route} redirect`).toBe(303);
  await atPath(page, `/${lang}/login`);
});

test('5.4 Private Game projections show their unavailable state', { tag: '@mail' }, async ({ mail, page, lang, account }) => {
  await verifiedLogin(page, lang, account, mail);
  for (const [path, status] of [
    ['account/dashboard', 503],
    ['account/journal/reports', 503],
    ['account/game-logins', 503],
    ['account/characters/new', 404],
    ['account/privacy/delete', 404],
  ]) {
    const route = `/${lang}/${path}`;
    await test.step(`GET ${route}`, async () => {
      await visit(page, route, status);
      await localized(page, lang);
    });
  }
});
