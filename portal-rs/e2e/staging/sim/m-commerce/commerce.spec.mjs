import { test, expect } from '@playwright/test';
import {
  AccountSetupUnavailable, origin, ready, registerPlayer, resetStaff, stagingBrowser, stagingRequest,
} from './helpers.mjs';

const id = '019a0000-0000-7000-8000-000000000001';
const native = '/api/v1/native-account/coin-purchases';
const checkout = '/native-account/coin-checkout/{id}';
const products = '/internal/v1/products-entitlements';
const wallet = '/internal/v1/coins';
const bazaar = '/internal/v1/bazaar';
const itemActions = ['bid', 'offer', 'accept', 'reject', 'buy-now', 'publish', 'cancel', 'renew'];

const routes = {
  Store: [
    ['GET', '/{lang}/shop', 404, 'page'],
    ['POST', '/{lang}/shop', 404, 'form'],
    ['GET', '/{lang}/shop/coins', 404, 'page'],
    ['POST', '/{lang}/shop/coins', 404, 'form'],
    ['GET', '/{lang}/shop/history', 404, 'page'],
    ['GET', '/{lang}/shop/transfer', 404, 'page'],
    ['POST', '/{lang}/shop/transfer', 404, 'form'],
    ['GET', '/{lang}/shop/test-checkout/{id}', 404, 'page'],
    ['POST', '/{lang}/shop/test-checkout/{id}', 404, 'form'],
    ['GET', '/{lang}/admin/store/codes', 404, 'page'],
    ['POST', '/{lang}/admin/store/codes', 404, 'form'],
    ['POST', '/{lang}/shop/codes/redeem', 404, 'json'],
    ...['balance/read', 'history/read', 'purchases', 'operations/pending', 'operations/read',
      'operations/receipt', 'operations/reconcile'].map(path =>
      ['POST', `${products}/store/${path}`, 404, 'private']),
    ...['premium-snapshots/read', 'loyalty-snapshots/read', 'loyalty-snapshots/stream',
      'profile-b/authority/read', 'profile-b/changes/read'].map(path =>
      ['POST', `${products}/${path}`, 404, 'private']),
  ],
  Wallet: [
    ...['transfers', 'market/offers', 'market/settlements', 'market/cancellations'].map(path =>
      ['POST', `${wallet}/${path}`, 404, 'private']),
    ...['operations/{id}', 'accounts/{id}/balance', 'accounts/{id}/history'].map(path =>
      ['GET', `${wallet}/${path}`, 404, 'private']),
  ],
  Payments: [
    ['GET', '/{lang}/panel/premium', 404, 'page'],
    ['GET', '/{lang}/panel/payments', 404, 'page'],
    ['GET', '/{lang}/panel/payments/receipts/{id}', 404, 'page'],
    ['GET', `${native}/catalogue`, 503, 'status'],
    ['GET', `${native}/balance`, 503, 'status'],
    ['POST', `${native}/orders`, 503, 'json'],
    ['GET', `${native}/requests/{id}`, 503, 'status'],
    ['GET', checkout, 503, 'page'],
    ['POST', checkout, 503, 'form'],
    ['POST', '/api/v1/payments/provider-webhook', 503, 'json'],
    ['POST', '/__test/payments/webhook', 404, 'private'],
  ],
  Bazaar: [
    ['GET', '/{lang}/bazaar', 503, 'characters'],
    ['GET', '/{lang}/account/bazaar', 503, 'characters'],
    ['GET', '/{lang}/bazaar/{id}', 503, 'characters'],
    ['GET', '/{lang}/account/bazaar/sell', 503, 'characters'],
    ['POST', '/{lang}/account/bazaar/sell', 405, 'form'],
    ['POST', '/{lang}/account/bazaar/{id}/bids', 404, 'form'],
    ['POST', '/{lang}/account/bazaar/{id}/control', 404, 'form'],
    ['GET', '/{lang}/bazaar/items', 503, 'items'],
    ['POST', '/{lang}/bazaar/items', 503, 'items'],
    ['GET', '/{lang}/bazaar/items/new', 503, 'items'],
    ['GET', '/{lang}/bazaar/items/{id}', 503, 'items'],
    ...itemActions.map(action => ['POST', `/{lang}/bazaar/items/{id}/${action}`, 503, 'items']),
    ['GET', '/{lang}/bazaar/sellers/{id}', 404, 'page'],
    ['GET', '/{lang}/account/bazaar/{id}/report', 404, 'page'],
    ['POST', '/{lang}/account/bazaar/{id}/report', 404, 'form'],
    ['GET', '/{lang}/account/bazaar/{id}/promote', 404, 'page'],
    ['POST', '/{lang}/account/bazaar/{id}/promote', 404, 'form'],
    ['GET', '/admin/{lang}/bazaar', 404, 'page'],
    ['POST', '/admin/{lang}/bazaar', 404, 'form'],
    ...['commands/pending', 'commands/read', 'commands/reconcile', 'transferability',
      'status', 'watermark'].map(path => ['POST', `${bazaar}/${path}`, 404, 'private']),
    ['POST', '/internal/v1/game-auth/character-commands/receipt', 404, 'private'],
    ['POST', '/internal/v1/game-events', 404, 'private'],
  ],
  'Staff commerce': [
    ['GET', '/admin/commerce', 404, 'page'],
    ...['grant', 'revoke', 'adjust', 'reconcile', 'refund', 'approvals/{id}',
      'cancellations/{id}'].map(path => ['POST', `/admin/commerce/${path}`, 404, 'form']),
  ],
};

const copy = {
  en: {
    characters: 'Accepted character custody and signing authority are unavailable. No listing, bid or transfer can be accepted.',
    title: 'Character auctions unavailable',
    items: 'Item trading is temporarily unavailable. Deposits and reserved funds remain protected.',
    notFound: 'Page not found',
    methodNotAllowed: 'Method not allowed',
  },
  pl: {
    characters: 'Brak zatwierdzonego źródła uprawnień do depozytu i podpisu postaci. Wystawienie, licytacja i przeniesienie są niedostępne.',
    title: 'Aukcje postaci są niedostępne',
    items: 'Handel przedmiotami jest tymczasowo niedostępny. Depozyty i zarezerwowane środki pozostają zabezpieczone.',
    notFound: 'Nie znaleziono strony',
    methodNotAllowed: 'Niedozwolona metoda',
  },
};

async function checkHTTP(request, lang, route, variant, body, observed, signedIn) {
  const [method, pattern, expected, surface] = route;
  const path = pattern.replace('{lang}', lang).replace('{id}', variant === 'invalid id' ? 'invalid' : id);
  const label = `${method} ${pattern} (${variant})`;
  const response = await stagingRequest(request, path, {
    method,
    headers: { 'accept-language': lang, ...(method === 'POST' ? { origin } : {}) },
    ...body,
  });
  try {
    const status = response.status();
    // The shared chrome redirects anonymous GETs of account and panel URLs
    // before the router returns either a disabled page or its fallback.
    const loginRequired = !signedIn && method === 'GET' && /^\/{lang}\/(account|panel)\//.test(pattern);
    const expectedStatus = loginRequired ? 303
      : variant === 'invalid id' && surface === 'characters' ? 404 : expected;
    if (variant === 'normal' || status !== expectedStatus) observed[label] = status;
    expect.soft(status, label).toBe(expectedStatus);
    const location = response.headers().location;
    if (loginRequired) {
      expect.soft(location === `/${lang}/login`, `${label} redirects to localized login`).toBe(true);
    } else {
      expect.soft(Boolean(location), `${label} stayed at the requested route`).toBe(false);
    }
    if (status === 404 || status === 405 || status === 503) {
      const text = await response.text();
      const main = text.match(/<main\b[^>]*>([\s\S]*?)<\/main>/i)?.[1] ?? '';
      expect.soft(/<form\b/i.test(main), `${label} has no commerce form`).toBe(false);
      if (status === 503 && (surface === 'characters' || surface === 'items')) {
        expect.soft(text.includes(copy[lang][surface]), `${label} localized refusal`).toBe(true);
      } else {
        const heading = status === 404 ? copy[lang].notFound : copy[lang].methodNotAllowed;
        const isRefusal = !text.trim() || (status !== 503 && main.includes(heading));
        expect.soft(isRefusal, `${label} contains only the disabled response`).toBe(true);
      }
    }
  } finally {
    await response.dispose();
  }
}

async function checkModule(request, lang, module, info, actor = 'anonymous') {
  const observed = {};
  try {
    for (const route of routes[module]) {
      const [method, path, , surface] = route;
      const signedIn = actor !== 'anonymous';
      await checkHTTP(request, lang, route, 'normal', {}, observed, signedIn);
      if (path.includes('{id}')) await checkHTTP(request, lang, route, 'invalid id', {}, observed, signedIn);
      if (method !== 'POST' || surface === 'private') continue;
      // Disabled mutations have no usable form or record. Repeat the empty request,
      // then submit a wrong type and an oversized field with no actionable values.
      await checkHTTP(request, lang, route, 'repeat empty submission', {}, observed, signedIn);
      const options = surface === 'json' ? { data: { request_id: [] } } : { form: { request_id: 'invalid' } };
      await checkHTTP(request, lang, route, 'invalid field', options, observed, signedIn);
      const large = 'a'.repeat(8_193);
      await checkHTTP(request, lang, route, 'long field',
        surface === 'json' ? { data: { request_id: large } } : { form: { request_id: large } }, observed, signedIn);
    }
  } finally {
    await info.attach(`${module} HTTP statuses (${actor}, ${lang})`, {
      contentType: 'application/json',
      body: JSON.stringify(observed),
    });
  }
}

async function unavailableBazaar(page, lang) {
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.getByRole('heading', { name: copy[lang].title, exact: true })).toBeVisible();
  await expect(page.getByText(copy[lang].characters, { exact: true })).toBeVisible();
  await expect(page.locator('main form')).toHaveCount(0);
}

async function navigation(page, lang, info) {
  const landing = `/${lang}/news`;
  const path = `/${lang}/bazaar`;
  expect((await page.goto(landing, { waitUntil: 'domcontentloaded' })).status()).toBe(200);
  const nav = page.locator('#portal-navigation');
  for (const disabled of ['shop', 'panel/premium', 'panel/payments']) {
    await expect.soft(page.locator(`#portal-header a[href="/${lang}/${disabled}"], #podglad-konta a[href="/${lang}/${disabled}"]`)).toHaveCount(0);
  }
  const mobile = page.locator('#portal-header details.om-fallback-menu > summary');
  if (await mobile.isVisible()) await mobile.click({ timeout: 5_000 });
  const link = nav.locator(`a[href="${path}"]`).first();
  const group = page.locator('#portal-navigation details').filter({ has: page.locator(`a[href="${path}"]`) }).first();
  if (!await link.isVisible()) await group.locator(':scope > summary').click({ timeout: 5_000 });
  await expect(link).toHaveAttribute('href', path);
  const [clicked] = await Promise.all([
    page.waitForResponse(response => response.request().isNavigationRequest() &&
      new URL(response.url()).origin === origin && new URL(response.url()).pathname === path, { timeout: 20_000 }),
    link.click({ timeout: 5_000 }),
  ]);
  try {
    expect(clicked.status(), 'Bazaar reached from the navigation').toBe(503);
    await unavailableBazaar(page, lang);
    expect((await page.reload()).status()).toBe(503);
    await unavailableBazaar(page, lang);
    await page.goBack();
    await expect(page).toHaveURL(landing);
    const otherLang = lang === 'pl' ? 'en' : 'pl';
    const switchLink = page.locator(`footer a[hreflang="${otherLang}"]`);
    await expect(switchLink).toHaveAttribute('href', `/${otherLang}/news`);
    await switchLink.click();
    await expect(page.locator('html')).toHaveAttribute('lang', otherLang);
    const secondTab = await page.context().newPage();
    try {
      expect((await secondTab.goto(`/${otherLang}/bazaar`)).status()).toBe(503);
      await unavailableBazaar(secondTab, otherLang);
      expect((await secondTab.goto(`/${otherLang}/bazaar/items`)).status()).toBe(503);
      await expect(secondTab.getByRole('heading', { name: copy[otherLang].items, exact: true })).toBeVisible();
      await expect(secondTab.locator('main form')).toHaveCount(0);
      expect((await secondTab.goto(`/${otherLang}/shop`)).status()).toBe(404);
      await expect(secondTab.getByRole('heading', { name: copy[otherLang].notFound, exact: true })).toBeVisible();
    } finally {
      await secondTab.close();
    }
  } catch (error) {
    const current = new URL(page.url());
    if (current.origin === origin && current.pathname === path) {
      await page.screenshot({ path: info.outputPath('bazaar-unavailable.png'), fullPage: false }).catch(() => {});
    }
    throw error;
  }
}

test.beforeEach(async ({ request }) => {
  await ready(request);
});

for (const module of Object.keys(routes)) {
  test(`${module} disabled for anonymous visitors`, async ({ request }, info) => {
    await checkModule(request, info.project.metadata.lang, module, info);
  });
}

test('commerce navigation and localized Bazaar refusal', async ({ context, page }, info) => {
  await stagingBrowser(context);
  await navigation(page, info.project.metadata.lang, info);
});

test('commerce disabled for two fresh players', async ({ browser, context, page }, info) => {
  const lang = info.project.metadata.lang;
  await stagingBrowser(context);
  const { locale, viewport, hasTouch, isMobile, proxy } = info.project.use;
  const second = await browser.newContext({
    baseURL: origin, locale, viewport, hasTouch, isMobile, proxy, serviceWorkers: 'block',
  });
  try {
    await stagingBrowser(second);
    try {
      const secondPage = await second.newPage();
      await registerPlayer(page, lang, 1);
      await registerPlayer(secondPage, lang, 2);
      await navigation(page, lang, info);
      await navigation(secondPage, lang, info);
    } catch (error) {
      if (!(error instanceof AccountSetupUnavailable)) throw error;
      console.log(`Player setup blocked: ${error.message}`);
      test.skip(true, error.message);
    }
    for (const module of Object.keys(routes)) {
      await checkModule(page.request, lang, module, info, 'player 1');
      await checkModule(second.request, lang, module, info, 'player 2');
    }
  } finally {
    await second.close();
    await context.close();
  }
});

for (const role of ['platform-admin', 'moderator', 'support']) {
  test(`commerce disabled for staff ${role}`, async ({ context, page }, info) => {
    test.skip(info.project.name !== 'en-1440', 'Each shared staff account has one reset per suite');
    await stagingBrowser(context);
    try {
      try {
        await resetStaff(page, info.project.metadata.lang, role);
      } catch (error) {
        if (!(error instanceof AccountSetupUnavailable)) throw error;
        console.log(`Staff setup blocked: ${error.message}`);
        test.skip(true, error.message);
      }
      for (const lang of ['pl', 'en']) {
        for (const width of [1440, 390]) {
          await page.setViewportSize({ width, height: width === 390 ? 844 : 900 });
          await test.step(`${lang}-${width}`, async () => {
            await navigation(page, lang, info);
            for (const module of Object.keys(routes)) await checkModule(page.request, lang, module, info, role);
          });
        }
      }
    } finally {
      await context.close();
    }
  });
}
