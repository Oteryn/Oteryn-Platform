import {
  expect, fillRegistration, get, login, navigate, newcomer, openRegistration, origin,
  register, requireVerifiedPlayer, submitRegistration, test, ui,
} from './helpers.mjs';

test('registration validates mistakes and preserves anonymous consent flow', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const player = newcomer();
  const form = await openRegistration(page, lang);
  const button = form.locator('button[type=submit]');

  await fillRegistration(form, player, { email: 'forgot-the-at-sign' });
  await button.click();
  await expect(form.locator('[name=email]:invalid')).toBeFocused();
  await expect(page).toHaveURL(`${origin}/${lang}/register`);

  await fillRegistration(form, player, { terms: false, privacy: false });
  await button.click();
  await expect(form.locator('[name=terms]:invalid')).toBeFocused();
  await form.locator('[name=terms]').check();
  await button.click();
  await expect(form.locator('[name=privacy]:invalid')).toBeFocused();

  const terms = form.locator('a[href$="/legal/terms"]');
  await terms.click();
  await expect(page).toHaveURL(`${origin}/${lang}/legal/terms`);
  await expect(page.locator('main h1')).toHaveText(lang === 'pl' ? 'Regulamin Oteryn' : 'Terms of Service');
  await page.goBack({ waitUntil: 'domcontentloaded' });
  await expect(page.locator('main h1')).toHaveText(ui(lang)['register-title']);
  expect((await page.reload({ waitUntil: 'domcontentloaded' })).status()).toBe(200);

  const response = await submitRegistration(page, lang, player, { password: 'abc', confirmation: 'abcd' }, form);
  expect(response.status(), 'Short password and mismatch return the form').toBe(422);
  await expect(page.locator('main [role=alert]')).toHaveText(ui(lang)['register-errors']);
  await expect(page.locator('#password-error')).toHaveText(ui(lang)['register-password-error']);
  await expect(page.locator('#confirmation-error')).toHaveText(ui(lang)['register-confirm-error']);
  await expect(form.locator('[name=email]')).toHaveValue(player.email);
  await expect(form.locator('[name=password]')).toHaveValue('');
});

test('registration trims email and gives duplicate submissions the same inbox result', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const player = newcomer();
  // HTML strips the whitespace from a typed email. An uppercase replay also checks normalization.
  await register(page, lang, player, `  ${player.email}  `);
  await page.reload({ waitUntil: 'domcontentloaded' });
  await expect(page.locator('main [role=status]')).toHaveText(ui(lang)['verify-inbox']);
  const second = await page.context().newPage();
  try {
    await register(second, lang, player, player.email.toUpperCase());
    const refusal = await login(second, lang, player);
    expect(refusal.status()).toBe(302);
    await expect(second).toHaveURL(`${origin}/${lang}/login`);
    await expect(second.locator('main [role=alert]')).toBeVisible();
  } finally {
    await second.close();
  }
});

test('verification rejects bad input and consumes the emailed link once', async ({ page, request }, info) => {
  const { lang } = info.project.metadata;
  const player = newcomer();
  const bad = `/${lang}/verify-email?token=invalid`;
  expect((await navigate(page, bad)).status(), 'Malformed verification link').toBe(422);
  await expect(page.locator('main [role=status]')).toHaveText(ui(lang)['verify-invalid']);

  const link = await requireVerifiedPlayer(page, lang, player);
  expect((await navigate(page, link)).status(), 'Reopen the consumed link').toBe(422);
  await expect(page.locator('main [role=status]')).toHaveText(ui(lang)['verify-invalid']);
  const modified = new URL(link);
  const token = modified.searchParams.get('token');
  modified.searchParams.set('token', `${token[0] === '0' ? '1' : '0'}${token.slice(1)}`);
  expect((await navigate(page, modified)).status(), 'Edited link').toBe(422);
  await expect(page.locator('main [role=status]')).toHaveText(ui(lang)['verify-invalid']);

  const auth = await login(page, lang, player);
  expect(auth.status(), 'Sign in after verifying').toBe(302);
  await expect.soft(page, 'Newcomer lands on the account overview after sign-in').toHaveURL(`${origin}/account`);
  expect((await get(page.request, '/account')).status(), 'Authenticated overview after verification').toBe(200);
  expect((await get(request, '/account')).status(), 'The original anonymous context remains anonymous').toBe(303);
});
