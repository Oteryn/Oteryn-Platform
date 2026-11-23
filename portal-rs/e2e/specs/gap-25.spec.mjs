import { test, expect } from '../fixtures.mjs';
import { login, submit } from '../helpers.mjs';
import { messages, seedEditor, seedPlayer, visit } from '../public-depth-helpers.mjs';
import { assertShell } from '../shell.mjs';

test('players can start from home and find rules, answers and support', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const pl = lang === 'pl';
  await visit(page, `/${lang}/`, 200, { lang, noStore: false });
  const start = page.locator(`#portal-navigation a[href="/${lang}/start"]`);
  await expect(start).toHaveCount(1);
  if (info.project.use.viewport.width <= 960) await page.locator('#portal-menu-toggle').click();
  await start.click();
  await expect(page.locator('main h1')).toHaveText(pl ? 'Pierwsze kroki' : 'Getting started');
  await assertShell(page);

  for (const [route, heading] of [
    ['rules', pl ? 'Zasady społeczności' : 'Community rules'],
    ['server-information', pl ? 'Informacje o serwerach' : 'Server information'],
    ['faq', pl ? 'Najczęstsze pytania' : 'Frequently asked questions'],
    ['support', pl ? 'Pomoc dla graczy' : 'Player support'],
  ]) {
    await page.locator(`main article a[href="/${lang}/${route}"]`).click();
    await expect(page.locator('main h1')).toHaveText(heading);
    if (route === 'server-information') {
      await page.locator(`main a[href="/${lang}/worlds"]`).click();
      await expect(page).toHaveURL(new RegExp(`/${lang}/worlds$`));
      await expect(page.locator('html')).toHaveAttribute('lang', lang);
    } else if (route === 'faq') {
      await expect(page.locator(`footer a[href="/${lang}/faq"]`)).toHaveAttribute('aria-current', 'page');
      await assertShell(page);
      await page.locator(`main a[href="/${lang}/faq/account-access"]`).click();
      await expect(page.locator('main h1')).toHaveText(
        pl ? 'Jak odzyskać dostęp do konta?' : 'How do I recover access to my account?',
      );
      await expect(page.locator(`main a[href="/${lang}/forgot-password"]`)).toBeVisible();
      await page.locator(`main a[href="/${lang}/faq"]`).click();
      await expect(page.locator('main h1')).toHaveText(heading);
    } else if (route === 'support') {
      await expect(page.locator('main')).toContainText(pl ? 'komunikat o błędzie' : 'error message');
      await expect(page.locator(`main a[href="/${lang}/support/tickets/create"]`)).toBeVisible();
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await visit(page, `/${lang}/start`, 200, { lang });
  }
});

test('FAQ follows staff publication and ticket access stays in the account', async ({ page, browser }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const slug = `editorial-${crypto.randomUUID()}`;
  const faq = `/${lang}/faq`;
  const url = `${faq}/${slug}`;
  const title = { en: `Editor question ${slug}`, pl: `Pytanie redakcji ${slug}` };
  const answer = { en: 'Check the settings for your account.', pl: 'Sprawdź ustawienia swojego konta.' };
  const editor = await seedEditor(browser, info);
  const save = async (locale, body, published = '') => {
    await visit(editor.page, `/admin/pages/faq-${slug}/${locale}`, 200, { noStore: false });
    await submit(editor.page, 'main form', { title: title[locale], body, published_at: published });
    await expect(editor.page).toHaveURL(/\/admin\/pages$/);
  };
  try {
    await save('en', answer.en);
    await save('pl', answer.pl);
    await visit(page, faq, 200, { lang });
    await expect(page.locator(`main a[href="${url}"]`)).toHaveCount(0);
    await visit(page, url, 404);
    await expect(page.locator('main')).not.toContainText(title[lang]);
    await visit(page, `${faq}/Bad--Slug`, 404);

    const published = new Date(Date.now() - 60_000).toISOString();
    await save('en', answer.en, published);
    await save('pl', answer.pl, published);
    await visit(page, faq, 200, { lang });
    await page.getByRole('link', { name: title[lang], exact: true }).click();
    await expect(page.locator('main h1')).toHaveText(title[lang]);
    await expect(page.locator('main article')).toContainText(answer[lang]);
    await assertShell(page);

    await save('en', answer.en);
    await visit(page, url, 404);
    await visit(page, faq, 200, { lang });
    await expect(page.locator(`main a[href="${url}"]`)).toHaveCount(0);
  } finally {
    await editor.context.close();
  }

  const account = await seedPlayer(lang);
  await login(page, lang, account);
  const subject = `Account question ${crypto.randomUUID()}`;
  await visit(page, `/${lang}/support`, 200, { lang });
  await page.locator(`main a[href="/${lang}/support/tickets/create"]`).click();
  await submit(page, 'main form', { subject, body: 'I need help with my account.' });
  await expect(page).toHaveURL(/\/support\/tickets\/[\da-f-]+$/);
  await expect(page.locator('main')).toContainText(subject);

  const visitor = await browser.newContext({
    baseURL: new URL(page.url()).origin,
    ignoreHTTPSErrors: true,
    locale: lang,
    viewport: info.project.use.viewport,
  });
  try {
    const guest = await visitor.newPage();
    await visit(guest, `/${lang}/support`, 200, { lang });
    await expect(guest.locator('main')).not.toContainText(subject);
    await guest.goto(page.url());
    await expect(guest).toHaveURL(new RegExp(`/${lang}/login$`));
    await expect(guest.locator('main')).not.toContainText(subject);

    await login(guest, lang, await seedPlayer(lang));
    await visit(guest, new URL(page.url()).pathname, 404);
    await expect(guest.locator('main')).not.toContainText(subject);
  } finally {
    await visitor.close();
  }
});

test('i18n-parity for editorial and navigation messages', () => {
  for (const directory of ['templates/pages', 'crates/ui/i18n']) {
    const en = messages(directory, 'en');
    const pl = messages(directory, 'pl');
    if (directory === 'templates/pages') {
      expect(Object.keys(pl).sort()).toEqual(Object.keys(en).sort());
    }
    for (const key of directory === 'templates/pages'
      ? ['faq-title', 'faq-empty', 'faq-previous', 'faq-next', 'faq-back', 'faq-editor-help', 'next-page']
      : ['thais-nav-start', 'shell-faq']) {
      expect(en[key], `English ${key}`).toBeTruthy();
      expect(pl[key], `Polish ${key}`).toBeTruthy();
    }
  }
});
