import {test, expect} from '../fixtures.mjs';
import {csrfRejected, login, pool, register} from '../helpers.mjs';
import {assertShell} from '../shell.mjs';

test('the account journal shows one due reminder for a personal goal', async ({page, browser}, info) => {
  test.setTimeout(120_000);
  const lang = info.project.metadata.locale;
  const path = `/${lang}/account/journal`;
  const unavailable = lang === 'pl' ? 'Dane są niedostępne.' : 'Data is unavailable.';
  const account = await register(page, lang);
  await login(page, lang, account);
  await page.goto(`/${lang}/news`);

  const accountLink = page.locator('header [data-account-link]');
  const menu = page.locator('header details.om-fallback-menu:not([open]) > summary');
  if (!await accountLink.isVisible() && await menu.isVisible()) {
    await menu.click();
  }
  if (await accountLink.isVisible()) {
    await accountLink.click();
  }
  const fallback = page.locator('header details.om-fallback-account > summary');
  if (!await page.locator('#podglad-konta').isVisible() && await fallback.isVisible()) {
    await fallback.click();
  }
  const journalLink = page.locator(`#podglad-konta a[href="${path}"]:visible, header a[href="${path}"]:visible`).first();
  await expect(journalLink).toBeVisible();
  await journalLink.click();
  await expect(page).toHaveURL(new RegExp(`${path}$`));
  await assertShell(page);
  await expect(page.locator('[aria-labelledby="observation-death_recap"]')).toContainText(unavailable);
  await expect(page.locator('[aria-labelledby="observation-quest"]')).toContainText(unavailable);

  const db = pool();
  let today;
  try {
    ({rows: [{today}]} = await db.query("SELECT ((now() AT TIME ZONE 'UTC')::date)::text AS today"));
  } finally {
    await db.end();
  }

  const title = `<quest> ${'A'.repeat(85)} ${crypto.randomUUID().slice(0, 8)}`;
  const form = page.locator(`main form[method=post][action="${path}"]`);
  await form.locator('[name=title]').fill(title);
  await form.locator('[name=kind]').selectOption('quest');
  await form.locator('[name=remind_on]').fill(today);
  const payload = await form.evaluate(node => Object.fromEntries(new FormData(node)));
  const headers = {origin: new URL(page.url()).origin};
  await csrfRejected(page, path, payload);
  const invalid = await page.request.post(path, {headers, form: {...payload, remind_on: '2026-02-30'}});
  expect(invalid.status()).toBe(400);
  await form.getByRole('button').click();

  const saved = page.locator('article[data-goal]').filter({hasText: title});
  await expect(saved).toHaveCount(1);
  await expect(saved.locator('[data-reminder=due]')).toHaveCount(1);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  const repeat = await page.request.post(path, {headers, form: payload, maxRedirects: 0});
  expect(repeat.status()).toBe(303);
  await page.reload();
  await expect(saved).toHaveCount(1);
  await expect(saved.locator('[data-reminder=due]')).toHaveCount(1);

  const goalPath = await saved.locator('form').getAttribute('action');
  const foreignContext = await browser.newContext({
    baseURL: headers.origin, ignoreHTTPSErrors: true, viewport: info.project.use.viewport, locale: lang,
  });
  try {
    const foreign = await foreignContext.newPage();
    const other = await register(foreign, lang);
    await login(foreign, lang, other);
    expect((await foreign.goto(path)).status()).toBe(200);
    await expect(foreign.getByText(title, {exact: true})).toHaveCount(0);
    const csrf = await foreign.locator(`main form[action="${path}"] [name=_csrf]`).inputValue();
    const refusal = await foreign.request.post(goalPath, {headers, form: {_csrf: csrf, action: 'complete'}});
    expect(refusal.status()).toBe(404);
    expect((await foreign.request.get(`${path}/status`)).status()).toBe(403);
  } finally {
    await foreignContext.close();
  }

  await saved.locator('button[value=complete]').click();
  await expect(saved.locator('[data-reminder=due]')).toHaveCount(0);
  await expect(saved.locator('button[value=complete]')).toHaveCount(0);
});
