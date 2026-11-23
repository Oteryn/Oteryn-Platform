import { readFileSync } from 'node:fs';
import { withFallback } from '../bazaar/fallback.mjs';
import { expect, test } from '../fixtures.mjs';
import { password, seedPasswordRange } from '../helpers.mjs';
import { messages } from '../public-depth-helpers.mjs';

const mainHeading = page => page.getByRole('main').getByRole('heading', { level: 1 });
const mobile = info => info.project.use.viewport.width === 390;
const primaryNav = page => page.locator('#portal-navigation');

async function revealNav(page, info) {
  if (!mobile(info)) return;
  await page.locator('#portal-header').getByText('Menu', { exact: true }).filter({ visible: true }).click();
  await expect(primaryNav(page)).toBeVisible();
}

async function expectTargets(links) {
  expect(await links.count()).toBeGreaterThan(0);
  for (const link of await links.all()) {
    const bounds = await link.boundingBox();
    expect(bounds, await link.textContent()).toBeTruthy();
    expect(bounds.width).toBeGreaterThanOrEqual(44);
    expect(bounds.height).toBeGreaterThanOrEqual(44);
  }
}

test('sign-in, registration and verification use the same terms', async ({ page }, info) => {
  const { locale: lang } = info.project.metadata;
  const [signIn, createAccount, email] = lang === 'pl'
    ? ['Zaloguj się', 'Utwórz konto', 'Adres e-mail']
    : ['Sign in', 'Create an account', 'Email address'];

  await page.goto('/' + lang + '/');
  await revealNav(page, info);
  await primaryNav(page).getByRole('link', { name: signIn, exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`/${lang}/login$`));
  await expect(mainHeading(page)).toHaveText(signIn);
  await expect(page).toHaveTitle(`${signIn} · Oteryn`);
  await expect(page.getByRole('main').getByLabel(email, { exact: true })).toBeVisible();
  await expect(page.getByRole('main').getByRole('button', { name: signIn, exact: true })).toBeVisible();
  await revealNav(page, info);
  await expect(primaryNav(page).locator('.om-login')).toHaveAttribute('aria-current', 'page');

  await seedPasswordRange(password());
  await page.getByRole('main').getByRole('link', { name: createAccount, exact: true }).click();
  await expect(mainHeading(page)).toHaveText(createAccount);
  await expect(page.getByLabel(email, { exact: true })).toBeVisible();
  await expect(page.getByRole('main').getByRole('button', { name: createAccount, exact: true })).toBeVisible();
  if (mobile(info)) {
    const consent = page.locator('main label:has(input[type=checkbox])');
    await expect(consent).toHaveCount(2);
    await expectTargets(consent);
    await expectTargets(page.locator('main a[href*="/legal/"]'));
    await consent.first().click();
    await expect(consent.first().getByRole('checkbox')).toBeChecked();
  }

  await page.goto(`/${lang}/verify-email/resend`);
  await expect(page.getByRole('main').getByLabel(email, { exact: true })).toBeVisible();
});

test('home uses the page heading scale and matches its title', async ({ page }, info) => {
  const { locale: lang } = info.project.metadata;
  const title = messages('crates/ui/i18n', lang)['home-link'];
  await page.goto(`/${lang}/`);
  await expect(mainHeading(page)).toHaveText(title);
  await expect(page).toHaveTitle(`${title} · Oteryn`);
  const fontSize = await mainHeading(page).evaluate(node => parseFloat(getComputedStyle(node).fontSize));
  expect(fontSize).toBeGreaterThanOrEqual(mobile(info) ? 31 : 56);
});

test.describe('home navigation', () => {
  test.use({ hasTouch: true });

  test('switches groups and follows the getting-started link', async ({ page }, info) => {
    const { locale: lang } = info.project.metadata;
    await page.goto(`/${lang}/`);

    const toggle = page.locator('#portal-menu-toggle');
    const world = page.locator('#portal-world-button');
    const community = page.locator('#portal-community-button');
    const gettingStarted = primaryNav(page).locator('a[href="#jak-zaczac"]');
    if (mobile(info)) {
      await toggle.tap();
      await world.tap();
      await expect(world).toHaveAttribute('aria-expanded', 'true');
      await community.tap();
      await expect(toggle).toHaveAttribute('aria-expanded', 'true');
      await expect(world).toHaveAttribute('aria-expanded', 'false');
      await expect(community).toHaveAttribute('aria-expanded', 'true');
      await gettingStarted.tap();
      await expect(toggle).toHaveAttribute('aria-expanded', 'false');
    } else {
      await world.click();
      await expect(world).toHaveAttribute('aria-expanded', 'true');
      await community.click();
      await expect(community).toHaveAttribute('aria-expanded', 'true');
      await gettingStarted.click();
    }
    await expect(page).toHaveURL(/#jak-zaczac$/);
    await expect(page.locator('#jak-zaczac')).toBeFocused();
  });
});

test('shared footer leads to matching page headings and current links', async ({ page }, info) => {
  const { locale: lang } = info.project.metadata;
  await page.goto(`/${lang}/news`);
  const footer = page.locator('footer.foot');
  await expect(footer.locator(`a[href="/${lang}/news"]`)).toHaveAttribute('aria-current', 'page');
  if (mobile(info)) {
    await expectTargets(footer.locator('a'));
  }

  for (const path of ['today', 'rules', 'server-information', 'legal/terms']) {
    await page.goto(`/${lang}/news`);
    const link = footer.locator(`a[href="/${lang}/${path}"]`);
    const label = (await link.innerText()).trim();
    await link.click();
    await expect(mainHeading(page)).toHaveText(label);
  }

  await footer.locator(`a[href="/${lang}/help"]`).click();
  await expect(footer.locator(`a[href="/${lang}/help"]`)).toHaveAttribute('aria-current', 'page');
  await expect(footer.locator('a[href$="/report-a-bug"]')).not.toHaveAttribute('aria-current', 'page');
  if (mobile(info)) await expectTargets(page.locator('main li > a'));
});

test('standalone public pages carry one Oteryn title suffix', async ({ page }, info) => {
  const { locale: lang } = info.project.metadata;
  for (const path of ['news', 'events', 'help', 'today', 'legal/terms', 'legal/privacy',
    'legal/cookies', 'rules', 'server-information']) {
    await page.goto(`/${lang}/${path}`);
    expect(await page.title(), path).toMatch(/^.+ · Oteryn$/);
    expect((await page.title()).match(/ · Oteryn/g), path).toHaveLength(1);
  }
});

test('unavailable rankings identify the category and omit live filtering', async ({ page }, info) => {
  const { locale: lang } = info.project.metadata;
  const t = messages('templates/stats/rankings', lang);
  const response = await page.goto(`/${lang}/stats/rankings/yesterday`);
  expect(response.status()).toBe(503);
  const main = page.getByRole('main');
  await expect(main.getByRole('status')).toHaveText(t.unavailable);
  await expect(main.locator('[aria-current="page"]')).toHaveText(t.yesterday);
  await expect(main.locator('nav a, form')).toHaveCount(0);
});

test('delivery and league use their menu labels and suppress empty league sections', async ({ page, request }, info) => {
  test.setTimeout(90_000);
  const { locale: lang } = info.project.metadata;
  await page.goto(`/${lang}/`);
  await revealNav(page, info);
  await page.locator('#portal-world-button').click();
  const delivery = primaryNav(page).locator('a[href$="/atlas/delivery"]');
  const leagueLabel = (await primaryNav(page).locator('a[href$="/league"]').innerText()).trim();
  const deliveryLabel = (await delivery.innerText()).trim();
  await delivery.click();
  await expect(mainHeading(page)).toHaveText(deliveryLabel);

  const control = process.env.PORTAL_E2E_LEAGUE_CONTROL;
  expect(control).toBeTruthy();
  expect((await request.post(`${control}/pause`)).status()).toBe(200);
  try {
    await expect.poll(async () => (await request.get(`/${lang}/league`)).status(), {
      timeout: 45_000, intervals: [1000, 2000],
    }).toBe(503);
    const response = await page.goto(`/${lang}/league`);
    expect(response.status()).toBe(503);
    await expect(mainHeading(page)).toHaveText(leagueLabel);
    const main = page.getByRole('main');
    await expect(main.getByRole('status')).toHaveText(messages('crates/minigames/src/league', lang).unavailable);
    await expect(main.locator('h2, section')).toHaveCount(0);
  } finally {
    expect((await request.post(`${control}/resume`)).status()).toBe(200);
  }
});

test('unavailable Item Bazaar and Shop use shared page presentation', async ({ page }, info) => {
  const { locale: lang } = info.project.metadata;
  const alternate = lang === 'pl' ? 'en' : 'pl';
  const t = messages('crates/bazaar/src/items', lang);
  const admin = process.env.PORTAL_E2E_ADMIN_DATABASE_URL;
  expect(admin, 'disposable PostgreSQL administrator configuration').toBeTruthy();

  await withFallback(admin, async origin => {
    const bazaar = await page.goto(new URL(`/${lang}/bazaar/items`, origin).href);
    expect(bazaar.status()).toBe(503);
    await expect(mainHeading(page)).toHaveText(t.title);
    await expect(page.getByRole('main').getByRole('status')).toHaveText(t.unavailable);

    await page.goto(new URL(`/${lang}/`, origin).href);
    await revealNav(page, info);
    await primaryNav(page).locator('.om-shop').click();
    await expect(page).toHaveURL(new RegExp(`/static/thais/shop-${lang}[.]html$`));
    await expect(page.locator('#portal-header')).toHaveClass(/\bom-interior\b/);
    await expect(mainHeading(page)).toBeVisible();
    await expect(page.locator('main .shop-status')).toBeVisible();
    await revealNav(page, info);
    if (mobile(info)) {
      await expectTargets(page.locator('#portal-header .om-brand, #portal-header .om-fallback-menu > summary'));
      await expectTargets(primaryNav(page).locator('.om-tools a'));
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
    }
    await expect(primaryNav(page).locator('.om-shop')).toHaveAttribute('aria-current', 'page');
    await page.locator(`.foot-langs a[hreflang="${alternate}"]`).click();
    await expect(page.locator('html')).toHaveAttribute('lang', alternate);
    await expect(page).toHaveURL(new RegExp(`/static/thais/shop-${alternate}[.]html$`));
    await expect(page.getByRole('main').getByRole('link', {
      name: alternate === 'pl' ? 'Zaloguj się' : 'Sign in',
    })).toHaveAttribute('href', `/${alternate}/login`);
  });
});

test('i18n-parity for the UI consistency catalogs', () => {
  for (const dir of ['crates/ui/i18n', 'templates/account/login', 'templates/account/mfa',
    'templates/pages', 'crates/atlas/src/delivery', 'crates/minigames/src/league',
    'crates/bazaar/src/items', 'templates/stats/rankings']) {
    const keys = lang => [...readFileSync(new URL(`../../${dir}/${lang}.ftl`, import.meta.url), 'utf8')
      .matchAll(/^([\w-]+)\s*=/gm)].map(([, key]) => key).sort();
    expect(keys('pl'), dir).toEqual(keys('en'));
  }
});
