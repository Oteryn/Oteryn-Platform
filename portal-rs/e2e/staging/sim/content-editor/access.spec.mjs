import { expect, get, ready, register, submit, test, unique, verifyPlayer, visit } from './helpers.mjs';

const copy = {
  en: {
    news: 'News',
    missing: 'This page is currently unavailable.',
    login: 'Sign in',
    inbox: 'Check your inbox for an e-mail with the next steps.',
    registrationUnavailable: 'Registration is currently unavailable. Please try again later.',
    backToHome: 'Back to home',
    mfa: 'Multi-factor authentication required',
  },
  pl: {
    news: 'Aktualności',
    missing: 'Ta strona jest obecnie niedostępna.',
    login: 'Zaloguj się',
    inbox: 'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.',
    registrationUnavailable: 'Rejestracja jest obecnie niedostępna. Spróbuj ponownie później.',
    backToHome: 'Wróć na stronę główną',
    mfa: 'Wymagane uwierzytelnianie wieloskładnikowe',
  },
};
const editors = ['/admin/news/create', '/admin/pages', '/admin/wiki/articles/create'];

test.beforeEach(async ({ context }) => ready(context.request));

test.afterEach(async ({ page }, info) => {
  if (info.status !== 'failed' && info.status !== 'timedOut') return;
  const url = new URL(page.url());
  if (url.origin === 'https://staging.oteryn.com' && /^\/(pl|en)\/register(?:\/check-inbox)?$/.test(url.pathname)) {
    await page.screenshot({ path: info.outputPath('registration.png'), timeout: 5_000 });
  }
});

test('anonymous visitor switches news language and finds missing content', async ({ page }, info) => {
  const { locale } = info.project.metadata;
  const other = locale === 'pl' ? 'en' : 'pl';
  await visit(page, `/${locale}/news`);
  await expect(page.locator('html')).toHaveAttribute('lang', locale);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(copy[locale].news);
  await visit(page, `/${other}/news`);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(copy[other].news);
  expect((await page.goBack({ waitUntil: 'domcontentloaded' }))?.status()).toBe(200);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(copy[locale].news);
  await visit(page, `/${locale}/news/Bad--Slug`, 404);
  await visit(page, `/${locale}/pages/missing-${unique()}`, 404);
  expect((await page.reload({ waitUntil: 'domcontentloaded' }))?.status()).toBe(404);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(copy[locale].missing);
  await visit(page, '/rules');
  await expect(page).toHaveURL(/\/pl\/rules$/);
});

test('anonymous visitor cannot open an editor', async ({ page }, info) => {
  const { locale } = info.project.metadata;
  for (const path of editors) {
    const response = await get(page.request, path);
    if (process.env.OTERYN_SIM_ADMIN_STATUS === '404') {
      // The host guard runs before staff authentication.
      expect(response.status(), path).toBe(404);
      await visit(page, path, 404);
      expect(new URL(page.url()).pathname).toBe(path);
    } else {
      expect(response.status(), path).toBe(303);
      expect(response.headers().location).toBe('/login');
      await visit(page, path);
      await expect(page).toHaveURL(/\/(?:(?:pl|en)\/)?login$/);
      const formLanguage = await page.locator('html').getAttribute('lang');
      await expect(page.getByRole('heading', { level: 1 })).toHaveText(copy[formLanguage].login);
    }
    await expect(page.locator('main [name=slug], main [name=en_slug]')).toHaveCount(0);
  }
  await visit(page, `/${locale}/login`);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(copy[locale].login);
});

async function sendRegistration(page, locale, account, info) {
  const { email, password } = account;
  const status = await submit(page, 'main form', {
    email, password, password_confirmation: password, terms: true, privacy: true,
  }, 'button[type=submit]', 325_000);
  if (status === 503) {
    // The registration handler renders this status when a staging dependency is unavailable.
    await expect(page).toHaveURL(new RegExp(`/${locale}/register$`));
    await expect(page.locator('main').getByText(copy[locale].registrationUnavailable, { exact: true })).toBeVisible();
    const back = page.getByRole('link', { name: copy[locale].backToHome });
    await expect(back).toHaveAttribute('href', `/${locale}/`);
    await page.screenshot({ path: info.outputPath('registration-unavailable.png'), timeout: 5_000 });
    await page.goBack({ waitUntil: 'domcontentloaded' });
    await expect(page.locator('main form [name=email]')).toBeVisible();
    info.annotations.push({ type: 'staging limitation', description: 'Registration returned HTTP 503.' });
    return false;
  }
  expect(status, 'Registration response').toBe(303);
  await expect(page).toHaveURL(new RegExp(`/${locale}/register/check-inbox$`));
  await expect(page.getByRole('status')).toHaveText(copy[locale].inbox);
  return true;
}

test('fresh player sees registration validation and a localized submission outcome', async ({ page }, info) => {
  const { locale } = info.project.metadata;
  const account = await register(page, locale);
  const form = page.locator('main form');
  const email = form.locator('[name=email]');
  await email.fill('not-an-email');
  await form.locator('button[type=submit]').click();
  await expect(page).toHaveURL(new RegExp(`/${locale}/register$`));
  expect(await email.evaluate(input => input.validity.typeMismatch)).toBe(true);
  expect(await email.evaluate(input => input.validationMessage.length)).toBeGreaterThan(0);
  await expect(email).toBeFocused();
  if (!await sendRegistration(page, locale, account, info)) return;
  expect((await page.reload({ waitUntil: 'domcontentloaded' }))?.status()).toBe(200);
  await expect(page.getByRole('status')).toHaveText(copy[locale].inbox);
});

test('verified player account cannot enter the CMS', async ({ page }, info) => {
  const { locale } = info.project.metadata;
  const mail = Number(process.env.OTERYN_SIM_MAIL_STATUS);
  test.skip([404, 405].includes(mail), 'Staging Mailpit is unavailable for player email verification.');
  expect(mail, 'Mailpit availability').toBe(200);
  const account = await register(page, locale);
  const registered = await sendRegistration(page, locale, account, info);
  test.skip(!registered, 'Staging registration is unavailable.');
  await verifyPlayer(page, locale, account);
  test.skip(process.env.OTERYN_SIM_ADMIN_STATUS === '404', 'The staging host hides the CMS.');
  await expect(page.locator('form[action$="/logout"]')).toBeAttached();
  for (const path of editors) {
    await visit(page, path, 403);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(copy[locale].mfa);
    await expect(page.locator('main form')).toHaveCount(0);
  }
});

test('editor can recover access and open localized account controls', async ({ staffPage }, info) => {
  const { locale } = info.project.metadata;
  const { context, page } = await staffPage('editor', { needsPanel: false });
  try {
    await visit(page, `/${locale}/`);
    await expect(page.locator('html')).toHaveAttribute('lang', locale);
    await expect(page.locator('form[action$="/logout"]')).toBeAttached();
    await visit(page, `/${locale}/mfa`);
    await expect(page).toHaveURL(new RegExp(`/${locale}/mfa$`));
  } finally {
    await context.close();
  }
});

test('MFA-authenticated support staff cannot enter the CMS', async ({ staffPage }, info) => {
  const { locale } = info.project.metadata;
  const { context, page } = await staffPage('support');
  try {
    // /admin is allowed to the Support role and uses the same staff MFA guard.
    await visit(page, '/admin');
    await expect(page.locator('html')).toHaveAttribute('lang', locale);
    for (const path of editors) {
      await visit(page, path, 403);
      await expect(page.locator('main form')).toHaveCount(0);
    }
    await visit(page, '/admin');
  } finally {
    await context.close();
  }
});
