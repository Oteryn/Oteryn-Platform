import { expect, test } from '@playwright/test';
import { capture, crawl, save, summary } from './audit.mjs';
import { goto, protectContext, ready, submit, unique } from './helpers.mjs';

test.beforeEach(async ({ context }) => {
  await ready();
  await protectContext(context);
});

test('anonymous page inventory and visual consistency', async ({ page }, info) => {
  const language = info.project.metadata.language;
  const width = info.project.use.viewport.width;
  const results = [await crawl(page, info, 'anonymous', language, width)];
  if (width === 1440) results.push(await crawl(page, info, 'anonymous', language, 768));
  expect(summary(results), 'See per-page measurements and screenshots in test-results').toEqual([]);
});

test('sign-in form presents bad input and refusal where the user acted', async ({ page, context }, info) => {
  const lang = info.project.metadata.language;
  const url = `/${lang}/login`;
  const response = await goto(page, url);
  expect(response.status()).toBe(200);
  const heading = page.getByRole('heading', { level: 1 });
  await expect(heading).toHaveText(lang === 'pl' ? 'Zaloguj się' : 'Sign in');
  const form = page.locator('main form');
  const mfaLogin = (await form.locator('button:not([type])').count()) > 0;
  const expected = mfaLogin
    ? (lang === 'pl' ? 'Nie udało się zweryfikować żądania. Sprawdź dane i spróbuj ponownie.'
      : 'The request could not be verified. Check your details and try again.')
    : (lang === 'pl' ? 'Podane dane logowania są nieprawidłowe.' : 'The provided credentials are invalid.');
  const email = form.locator('[name=email]');
  await submit(form, { email: 'bad-address', password: 'invalid-example' });
  expect(await email.evaluate(el => el.validity.typeMismatch)).toBe(true);
  await expect(page.locator('main [role=alert]')).toHaveCount(0);
  const other = await context.newPage();
  await goto(other, url);
  const testEmail = `sim-visual-${unique()}-1@example.test`;
  await submit(form, { email: testEmail, password: 'invalid-example' });
  const alert = page.locator('main [role=alert]');
  await expect(alert).toHaveText(expected);
  await expect(alert).toBeInViewport();
  await expect(other.locator('main [role=alert]')).toHaveCount(0);
  const evidence = await capture(page, info, 'form-states', '/login-refusal');
  await save(info, 'form.json', evidence);
  const index = await alert.evaluate(el => {
    const h1 = el.parentElement.querySelector('h1');
    const form = el.parentElement.querySelector('form');
    return [h1?.nextElementSibling === el, el.nextElementSibling === form, el.classList.contains('op-error')];
  });
  expect(index).toEqual([true, true, true]);
  await goto(page, `/${lang}/forgot-password`);
  await page.goBack({ waitUntil: 'domcontentloaded' });
  await expect(heading).toHaveText(lang === 'pl' ? 'Zaloguj się' : 'Sign in');
  await other.close();
});

test('switching language preserves the news navigation layout', async ({ page }, info) => {
  const lang = info.project.metadata.language;
  const other = lang === 'en' ? 'pl' : 'en';
  await goto(page, `/${lang}/news`);
  const sample = async language => {
    const mobileMenu = page.locator('.om-fallback-menu > summary, #portal-menu-toggle').filter({ visible: true });
    const narrow = info.project.use.viewport.width <= 960;
    if (narrow) {
      await expect(mobileMenu.first()).toBeVisible();
      await mobileMenu.first().click();
    }
    const world = page.locator('#portal-world-button:visible, .om-section > summary:visible').first();
    await world.click();
    const picture = await capture(page, info, `language-menu/${language}`, '/open-news-menu', { states: false });
    if (narrow) await mobileMenu.first().click();
    const comparable = picture.signature.filter(([element]) => /om-top-link/.test(element));
    return { comparable, picture };
  };
  const here = await sample(lang);
  const switcher = page.locator(`footer a[hreflang=${other}]`).first();
  const sameSite = await switcher.evaluate(el => new URL(el.href).origin === location.origin);
  expect(sameSite).toBe(true);
  await switcher.click();
  await expect(page).toHaveURL(new RegExp(`/${other}/news(?:\\?|$)`));
  await page.waitForLoadState('domcontentloaded');
  const translated = await sample(other);
  await save(info, 'locale-pair.json', { here, translated });
  expect(translated.comparable.map(([selector]) => selector)).toEqual(here.comparable.map(([selector]) => selector));
  expect([...here.comparable, ...translated.comparable].filter(([, lines]) => lines > 1)).toEqual([]);
  const geometry = result => result.picture.problems.filter(problem => ['overlap', 'viewport', 'clipping'].includes(problem.kind));
  expect([...geometry(here), ...geometry(translated)]).toEqual([]);
  await page.goBack({ waitUntil: 'domcontentloaded' });
  await expect(page).toHaveURL(new RegExp(`/${lang}/news(?:\\?|$)`));
});
