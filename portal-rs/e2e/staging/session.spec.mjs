import { atPath, localized, password, test, visit } from './fixtures.mjs';
import { copy, login, logout, register, verifiedLogin } from './identity.mjs';

test('2.1 Login opens the account and logout redirects protected navigation to login', { tag: '@mail' }, async ({ mail, page, lang, account }) => {
  await verifiedLogin(page, lang, account, mail);
  await visit(page, `/${lang}/account`);
  await localized(page, lang, copy[lang].overview);
  await logout(page, lang);
  await visit(page, `/${lang}/account`, 200, { destination: `/${lang}/login` });
  await atPath(page, `/${lang}/login`);
});

test('2.2 Wrong password receives the generic refusal', { tag: '@mail' }, async ({ mail, page, lang, account }) => {
  await verifiedLogin(page, lang, account, mail);
  await logout(page, lang);
  await login(page, lang, { ...account, password: password() }, 'login');
});

test('2.3 Correct credentials for an unverified account are refused', async ({ page, lang, account }) => {
  await register(page, lang, account);
  await login(page, lang, account, 'login');
});
