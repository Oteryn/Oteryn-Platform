import { randomBytes } from 'node:crypto';
import { test as apiTest } from '@playwright/test';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, pool, submit } from '../helpers.mjs';
import { uuid7 } from '../game-sim-helpers.mjs';
import {
  bearer, callback, clientId, empty, fixture, gateway, json, nativeTls, origin, packageId, providerEvent,
  requestSource, seed, seedPlayer, sha256, waitForPortal,
} from '../gameapi-helpers.mjs';

const authorizePath = '/oauth/authorize';
const tokenPath = '/oauth/token';
const charactersPath = '/api/v1/game-auth/native-characters';
const ticketsPath = '/api/v1/game-auth/tickets';
const devicesPath = '/api/v1/game-auth/device-sessions';
const noStore = 'no-store, private';
const invalidRequest = {
  error: 'invalid_request',
  error_description: 'The request is missing a required parameter or is malformed.',
};
const invalidGrant = {
  error: 'invalid_grant',
  error_description: 'The provided authorization grant is invalid.',
};
const unauthenticated = { error: 'unauthenticated' };
const deviceDenied = { error: 'device_authorization_unavailable' };
const worldId = '0192a000-0000-7000-8000-000000000012';

const copy = {
  en: {
    title: 'Authorize game sign-in',
    ticket: 'Allow the registered game client to request a short-lived game login ticket for your account (game:ticket)?',
    services: 'Allow the registered game client to access account services, including coin purchases (game:account-services)?',
    approve: 'Allow', deny: 'Cancel', skip: 'Skip to content', home: 'Oteryn · Home',
  },
  pl: {
    title: 'Autoryzuj logowanie do gry',
    ticket: 'Czy zezwolić zarejestrowanemu klientowi gry na uzyskanie krótkotrwałego biletu logowania dla Twojego konta (game:ticket)?',
    services: 'Czy zezwolić zarejestrowanemu klientowi gry na dostęp do usług konta, w tym zakup monet (game:account-services)?',
    approve: 'Zezwól', deny: 'Anuluj', skip: 'Przejdź do treści', home: 'Oteryn · Strona główna',
  },
};

function authorization(scope, client = clientId) {
  const verifier = randomBytes(32).toString('base64url');
  const query = {
    response_type: 'code', client_id: client, redirect_uri: callback, scope,
    state: randomBytes(24).toString('hex'), code_challenge: sha256(verifier).digest('base64url'),
    code_challenge_method: 'S256',
  };
  return { verifier, query, url: `${authorizePath}?${new URLSearchParams(query)}` };
}

async function verifyConsent(page, lang, description) {
  const labels = copy[lang];
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page).toHaveTitle(`${labels.title} · Oteryn`);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(labels.title);
  await expect(page.locator('main p')).toHaveText(description);
  await expect(page.getByRole('button', { name: labels.approve, exact: true })).toHaveAttribute('value', 'approve');
  await expect(page.getByRole('button', { name: labels.deny, exact: true })).toHaveAttribute('value', 'deny');
  await expect(page.locator('main form')).toHaveAttribute('method', 'post');
  await expect(page.locator('main form')).toHaveAttribute('action', authorizePath);
  const home = page.locator('#portal-header a.om-brand');
  await expect(home).toHaveAccessibleName(labels.home);
  await expect(home).toHaveAttribute('href', `/${lang}/`);
  const skip = page.getByRole('link', { name: labels.skip, exact: true });
  await skip.focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('#main')).toBeFocused();
  for (const stylesheet of ['shell', 'interior']) {
    const url = `/static/thais/${stylesheet}.css`;
    await expect(page.locator(`link[href="${url}"]`)).toHaveCount(1);
    const response = await page.request.get(url);
    expect(response.status()).toBe(200);
    expect(response.headers()['content-type']).toContain('text/css');
  }
}

async function decide(page, grant, decision) {
  const redirect = page.waitForResponse(response =>
    new URL(response.url()).pathname === authorizePath && response.request().method() === 'POST');
  const received = page.waitForResponse(response => response.url().startsWith(callback + '?'));
  await page.locator(`main button[name=decision][value=${decision}]`).click();
  const result = await redirect;
  expect(result.status()).toBe(302);
  expect(result.headers()['cache-control']).toBe('no-store');
  const target = new URL(result.headers().location);
  expect(target.origin + target.pathname).toBe(callback);
  expect(target.searchParams.get('state') === grant.query.state).toBe(true);
  const delivery = await received;
  expect(delivery.status()).toBe(200);
  expect(delivery.headers()['cache-control']).toBe('no-store');
  expect(await delivery.text()).toBe('OAuth callback received.');
  return target.searchParams;
}

function tokenForm(grant, code) {
  return {
    grant_type: 'authorization_code', client_id: grant.query.client_id,
    redirect_uri: callback, code, code_verifier: grant.verifier,
  };
}

async function tokens(response, scope) {
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toBe('no-store');
  expect(response.headers().pragma).toBe('no-cache');
  expect(response.headers()['content-type']).toBe('application/json');
  const issued = await response.json();
  expect(Object.keys(issued).sort()).toEqual(['access_token', 'expires_in', 'refresh_token', 'scope', 'token_type']);
  expect(issued.token_type).toBe('bearer');
  expect(issued.expires_in).toBe(1800);
  expect(issued.scope).toBe(scope);
  const opaque = /^[A-Za-z0-9_-]{43}$/;
  expect(opaque.test(issued.access_token) && opaque.test(issued.refresh_token)).toBe(true);
  return issued;
}

async function grantAccess(page, request, scope, headers, locale) {
  const grant = authorization(scope);
  expect((await page.goto(grant.url)).status()).toBe(200);
  if (locale) await verifyConsent(page, locale, copy[locale].services);
  const values = await decide(page, grant, 'approve');
  const issued = await tokens(await request.post(tokenPath, { form: tokenForm(grant, values.get('code')), headers }), scope);
  return issued.access_token;
}

async function signIn(page, lang, player, returnTo = `/${lang}/`) {
  expect((await page.goto(`/${lang}/login`)).status()).toBe(200);
  await submit(page, 'main form', { email: player.email, password: player.password });
  await expect(page).toHaveURL(origin + returnTo);
}

async function publishCharacter(request, player, other) {
  const character = uuid7();
  const seeded = await request.post('/__bazaar_game/seed', { data: {
    seller: other.accountId, buyer: player.accountId, kind: 'item',
    asset_id: uuid7(), seller_character_id: uuid7(), buyer_character_id: character,
  } });
  expect(seeded.status()).toBe(201);
  return character;

}

test.beforeAll(async ({ request }) => {
  await waitForPortal(request);
  expect(seed('oauth-native-client', { clientId }).clientId).toBe(clientId);
});

test('OAuth consent links, both decisions, CSRF, PKCE, code replay and refresh rotation', async ({ page, request }, info) => {
  const lang = info.project.metadata.locale;
  const headers = requestSource(info, 1);
  await page.setExtraHTTPHeaders(headers);
  const grant = authorization('game:ticket');
  const guest = await request.get(grant.url, { headers, maxRedirects: 0 });
  expect(guest.status()).toBe(303);
  expect(guest.headers().location).toBe(`/${lang}/login`);
  await json(await request.get(authorizePath, { headers }), 400, invalidRequest);
  await page.goto(grant.url);
  await expect(page).toHaveURL(`${origin}/${lang}/login`);
  const guestCsrf = await page.locator('main form input[name=_csrf]').inputValue();
  await json(await page.request.post(authorizePath, {
    form: { ...grant.query, _csrf: guestCsrf, decision: 'approve' }, headers: { ...headers, origin },
  }), 400, invalidGrant);
  await signIn(page, lang, seedPlayer(lang), grant.url);

  expect((await page.goto(grant.url)).status()).toBe(200);
  await verifyConsent(page, lang, copy[lang].ticket);
  await page.locator('#portal-header a.om-brand').click();
  await expect(page).toHaveURL(`${origin}/${lang}/`);
  expect((await page.goto(grant.url)).status()).toBe(200);

  const form = await page.locator('main form').evaluate(element => Object.fromEntries(new FormData(element)));
  const fields = { ...form };
  delete fields._csrf;
  expect(fields).toEqual(grant.query);
  await empty(await page.request.post(authorizePath, {
    form: { ...fields, decision: 'approve' }, headers: { ...headers, origin },
  }), 403, 'no-store');
  await json(await page.request.post(authorizePath, {
    form: { ...form, decision: 'unknown' }, headers: { ...headers, origin },
  }), 400, invalidRequest);
  const denied = await decide(page, grant, 'deny');
  expect(denied.get('error')).toBe('access_denied');
  expect(denied.has('code')).toBe(false);

  expect((await page.goto(grant.url)).status()).toBe(200);
  const approved = await decide(page, grant, 'approve');
  expect(approved.has('error')).toBe(false);
  expect(approved.has('code')).toBe(true);
  const exchange = tokenForm(grant, approved.get('code'));
  const token = form => request.post(tokenPath, { form, headers });
  await json(await token({ ...exchange, code_verifier: randomBytes(32).toString('base64url') }), 400, invalidGrant);
  const issued = await tokens(await token(exchange), grant.query.scope);
  await json(await token(exchange), 400, invalidGrant);

  const refresh = { grant_type: 'refresh_token', client_id: clientId, refresh_token: issued.refresh_token };
  await json(await token({ ...refresh, scope: 'game:account-services' }), 400, invalidGrant);
  const rotated = await tokens(await token(refresh), grant.query.scope);
  expect(rotated.refresh_token !== issued.refresh_token && rotated.access_token !== issued.access_token).toBe(true);
  await json(await token(refresh), 400, invalidGrant);
});

apiTest('OAuth token rejects malformed, oversized, unknown client and unsupported grants', async ({ request }, info) => {
  const headers = requestSource(info, 2);
  await json(await request.post(tokenPath, { headers, data: '{}' }), 400, invalidRequest);
  await json(await request.post(tokenPath, {
    headers: { ...headers, 'content-type': 'application/x-www-form-urlencoded' },
    data: 'grant_type=' + 'x'.repeat(16_384),
  }), 400, invalidRequest);
  await json(await request.post(tokenPath, { headers, form: { grant_type: 'password', client_id: 'unknown-client' } }), 401, {
    error: 'invalid_client', error_description: 'Client authentication failed.',
  });
  await json(await request.post(tokenPath, { headers, form: { grant_type: 'password', client_id: clientId } }), 400, {
    error: 'unsupported_grant_type',
    error_description: 'The authorization grant type is not supported by the authorization server.',
  });
});


apiTest('OAuth token throttles its source and returns the shared refusal envelope', async ({ request }, info) => {
  const headers = requestSource(info, 3);
  await expect.poll(() => new Date().getUTCSeconds(), { timeout: 65_000 }).toBeLessThan(20);
  const form = { grant_type: 'password', client_id: clientId };
  const body = {
    error: 'unsupported_grant_type',
    error_description: 'The authorization grant type is not supported by the authorization server.',
  };
  for (let offset = 0; offset < 20; offset += 5) {
    await Promise.all(Array.from({ length: 5 }, async () => {
      await json(await request.post(tokenPath, { headers, form }), 400, body);
    }));
  }
  const limited = await request.post(tokenPath, { headers, form });
  await json(limited, 429, { status: 429, error: 'Zbyt wiele żądań' });
  expect(limited.headers()['cache-control']).toBe('no-store');
  expect(Number(limited.headers()['retry-after'])).toBeGreaterThanOrEqual(1);
  expect(Number(limited.headers()['retry-after'])).toBeLessThanOrEqual(60);
});

test('native bearer reads projected characters, validates and consumes a ticket, and enforces both budgets', async ({ page, request }, info) => {
  const lang = info.project.metadata.locale;
  const headers = requestSource(info, 4);
  await page.setExtraHTTPHeaders(headers);
  const player = seedPlayer(lang);
  const other = seedPlayer(lang);
  await signIn(page, lang, player);
  const access = await grantAccess(page, request, 'game:ticket', headers);
  const character = await publishCharacter(request, player, other);

  const body = { protocol_version: 2, characters: [{
    character_id: character, world_id: worldId, name: `Fixture ${character.slice(-12)}`, availability: 'AVAILABLE',
  }] };
  const validHeaders = bearer(access);
  await json(await page.request.get(charactersPath), 401, unauthenticated);
  await json(await request.get(charactersPath, { headers: bearer('unknown') }), 401, unauthenticated);
  const list = await request.get(charactersPath, { headers: validHeaders });
  await json(list, 200, body);
  expect(list.headers()['cache-control']).toBe(noStore);
  await Promise.all(Array.from({ length: 9 }, async () => {
    await json(await request.get(charactersPath, { headers: validHeaders }), 200, body);
  }));
  const readLimit = await request.get(charactersPath, { headers: validHeaders });
  await empty(readLimit, 429, noStore);
  expect(readLimit.headers()['retry-after']).toBe('30');

  const issue = data => request.post(ticketsPath, { headers: validHeaders, data });
  await json(await page.request.post(ticketsPath, { data: { protocol_version: 1 } }), 401, unauthenticated);
  expect((await issue(' '.repeat(2049))).status()).toBe(413);
  for (const payload of [{ protocol_version: 2 }, { protocol_version: 1, unknown: true }, {}]) {
    await json(await issue(payload), 422, { error: 'invalid_request' });
  }
  const response = await issue({ protocol_version: 1 });
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toBe(noStore);
  const ticket = await response.json();
  expect(Object.keys(ticket).sort()).toEqual(['expires_in', 'protocol_version', 'ticket']);
  expect(ticket.expires_in).toBe(60);
  expect(ticket.protocol_version).toBe(1);
  expect(/^[A-Za-z0-9_-]{43}$/.test(ticket.ticket)).toBe(true);
  const loginWire = JSON.parse(fixture('gateway/tests/fixtures/game_client_native_login/request.json'));
  const admit = async (character_id, status, code, public_class) => {
    const attempt_ref = uuid7();
    await json(await request.post(gateway + '/v1/login', { headers, data: {
      ...loginWire, game_login_ticket: ticket.ticket, attempt_ref, character_id,
    } }), status, { protocol_version: 2, error: { code, public_class }, attempt_ref });
  };
  await admit(uuid7(), 409, 'NATIVE_LOGIN_CHARACTER_CONFLICT', 'SESSION_UNAVAILABLE');
  // The refused attempt must roll back ticket consumption. The same ticket
  // reaches the next gate for the actual owner, where signing custody is absent.
  await admit(character, 503, 'NATIVE_LOGIN_UNAVAILABLE', 'TEMPORARILY_UNAVAILABLE');
  await json(await issue({ protocol_version: 1 }), 401, unauthenticated);
  const fresh = await grantAccess(page, request, 'game:ticket', headers);
  await Promise.all(Array.from({ length: 5 }, async () => {
    await json(await request.post(ticketsPath, { headers: bearer(fresh), data: {} }), 422, { error: 'invalid_request' });
  }));
  const issueLimit = await request.post(ticketsPath, { headers: bearer(fresh), data: { protocol_version: 1 } });
  await empty(issueLimit, 429, noStore);
  expect(issueLimit.headers()['retry-after']).toBe('30');
});

test('remembered device requires direct TLS and rotates and revokes a seeded native session', async ({ page, request }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const headers = requestSource(info, 6);
  await page.setExtraHTTPHeaders(headers);
  const player = seedPlayer(lang);
  const other = seedPlayer(lang);
  await signIn(page, lang, player);
  const access = await grantAccess(page, request, 'game:ticket', headers);
  const enrollment = { protocol_version: 1, remember_device: true };
  const options = { data: enrollment, headers: bearer(access) };
  await json(await request.post(devicesPath, options), 503, deviceDenied);
  const direct = nativeTls + devicesPath;
  const db = pool();
  try {
    await expect.poll(async () => (await db.query(
      'SELECT coalesce(max(attempts),0) AS used FROM device_session_source_limits WHERE window_start > floor(extract(epoch FROM clock_timestamp())) - 60',
    )).rows[0].used, { timeout: 65_000, intervals: [1000] }).toBeLessThanOrEqual(18);
  } finally {
    await db.end();
  }
  await json(await request.post(direct, { data: enrollment }), 401, deviceDenied);
  await json(await request.post(direct, { ...options, data: { ...enrollment, remember_device: false } }), 400, { error: 'invalid_request' });
  await json(await request.post(direct, { ...options, data: ' '.repeat(1025) }), 413, { error: 'invalid_request' });
  await json(await request.post(direct + '/unknown', options), 503, deviceDenied);

  const enrolled = await request.post(direct, options);
  expect(enrolled.status()).toBe(200);
  expect(enrolled.headers()['cache-control']).toBe(noStore);
  const device = await enrolled.json();
  expect(device.protocol_version).toBe(1);
  expect(Number.isInteger(device.absolute_expires_at) && device.absolute_expires_at >= device.idle_expires_at).toBe(true);
  await json(await request.post(direct, options), 401, deviceDenied);

  const operate = (suffix, credential) => request.post(direct + suffix, {
    headers: { authorization: `OterynDevice ${credential}` },
    data: { protocol_version: 1, client_id: clientId },
  });
  // No portfolio is published for this owner, even when another test opened the feed.
  await json(await operate('/native-characters', device.device_credential), 503, deviceDenied);
  const character = await publishCharacter(request, player, other);
  const projected = await operate('/native-characters', device.device_credential);
  expect(projected.status()).toBe(200);
  const updated = await projected.json();
  expect(updated.characters).toEqual([{
    character_id: character, world_id: worldId, name: `Fixture ${character.slice(-12)}`, availability: 'AVAILABLE',
  }]);
  expect(updated.device_credential !== device.device_credential && updated.family_id === device.family_id).toBe(true);
  const issued = await operate('/tickets', updated.device_credential);
  expect(issued.status()).toBe(200);
  const rotated = await issued.json();
  expect(rotated.family_id === device.family_id && rotated.device_credential !== device.device_credential).toBe(true);
  expect(rotated.expires_in).toBe(60);
  expect(/^[A-Za-z0-9_-]{43}$/.test(rotated.ticket)).toBe(true);
  await json(await operate('/tickets', device.device_credential), 401, deviceDenied);
  for (let attempt = 0; attempt < 2; attempt++) {
    await json(await operate('/revoke', rotated.device_credential), 200, { protocol_version: 1, revoked: true });
  }
  await json(await operate('/tickets', rotated.device_credential), 401, deviceDenied);

  let limited;
  for (let offset = 0; offset < 35 && !limited; offset += 5) {
    const responses = await Promise.all(Array.from({ length: 5 }, () => request.post(direct, { data: enrollment })));
    for (const response of responses) {
      if (response.status() === 429) {
        await json(response, 429, { error: 'too_many_requests' });
        expect(response.headers()['retry-after']).toBe('60');
        limited = response;
      } else {
        await json(response, 401, deviceDenied);
      }
    }
  }
  expect(limited, 'the dedicated TLS listener must enforce its source budget').toBeDefined();
});

const checkoutCopy = {
  en: {
    title: 'Sandbox coin checkout',
    note: 'Test payment only. No real money is charged. Coins are credited only after the payment is verified.',
    labels: ['Order', 'Coins', 'Price', 'Status'],
    awaiting: 'Awaiting payment', paid: 'Paid', failed: 'Failed', refunded: 'Refunded', chargeback: 'Charged back', processing: 'Processing',
    pay: 'Complete test payment', fail: 'Fail test payment',
    done: 'You can return to the game. It reads the order status and your coin balance from Oteryn.',
  },
  pl: {
    title: 'Testowy zakup monet',
    note: 'To płatność testowa. Nie pobieramy prawdziwych pieniędzy. Monety zostaną dodane dopiero po weryfikacji płatności.',
    labels: ['Zamówienie', 'Monety', 'Cena', 'Status'],
    awaiting: 'Oczekuje na płatność', paid: 'Opłacone', failed: 'Nieudane', refunded: 'Zwrócone', chargeback: 'Płatność cofnięta', processing: 'Przetwarzane',
    pay: 'Zakończ płatność testową', fail: 'Odrzuć płatność testową',
    done: 'Możesz wrócić do gry. Gra odczyta stan zamówienia i saldo monet z Oteryn.',
  },
};

async function providerReference(orderId) {
  const db = pool();
  try {
    const result = await db.query(
      'SELECT p.provider_reference FROM coin_purchase_orders o JOIN payment_intents p ON p.id=o.payment_intent_id WHERE o.id=$1',
      [orderId],
    );
    expect(result.rowCount).toBe(1);
    return result.rows[0].provider_reference;
  } finally {
    await db.end();
  }
}

function reversal(type, order_reference, minor_units = 499) {
  return {
    version: 1, event_id: uuid7(), type, order_reference,
    amount: { minor_units, currency: 'PLN' },
  };
}

test('native coin catalogue, balance, order replay and both sandbox checkout actions', async ({ page, request, browser }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const headers = requestSource(info, 8);
  await page.setExtraHTTPHeaders(headers);
  const player = seedPlayer(lang);
  const other = seedPlayer(lang);
  await signIn(page, lang, player);
  const access = await grantAccess(page, request, 'game:account-services', headers, lang);
  const auth = bearer(access);
  const owned = await publishCharacter(request, player, other);
  const api = '/api/v1/native-account/coin-purchases';
  const get = path => request.get(api + path, { headers: auth });
  const post = data => request.post(api + '/orders', { headers: auth, data });
  const orderStatus = id => get('/requests/' + id);
  for (const [path, method] of [['/balance', 'GET'], ['/catalogue', 'GET'], ['/requests/' + uuid7(), 'GET'], ['/orders', 'POST']]) {
    const refused = await page.request.fetch(api + path, { method });
    await empty(refused, 401, 'no-store');
    expect(refused.headers()['www-authenticate']).toBe('Bearer');
  }
  await json(await request.get(charactersPath, { headers: auth }), 401, unauthenticated);
  for (const path of ['/catalogue?character_id=bad', '/balance?unexpected=1', '/requests/bad']) {
    await empty(await get(path), 400, 'no-store');
  }
  await empty(await get('/catalogue?character_id=' + uuid7()), 403, 'no-store');
  await empty(await orderStatus(uuid7()), 404, 'no-store');
  await empty(await post({ version: 1, request_id: uuid7() }), 400, 'no-store');
  await empty(await post(' '.repeat(8193)), 400, 'no-store');

  const balance = async (coins, reviewRequired = false) => {
    const response = await get('/balance');
    expect(response.status()).toBe(200);
    expect(response.headers()['cache-control']).toBe('no-store');
    expect(response.headers()['referrer-policy']).toBe('no-referrer');
    const body = await response.json();
    expect(body).toEqual({
      version: 1, revision: expect.any(Number), total_coins: coins,
      available_coins: coins, transferable_coins: coins, review_required: reviewRequired,
    });
    return body.revision;
  };
  const originalRevision = await balance(0);
  const catalogue = async character_id => {
    const response = await get('/catalogue' + (character_id ? '?character_id=' + character_id : ''));
    expect(response.status()).toBe(200);
    const body = await response.json();
    expect(body).toMatchObject({
      version: 1, sandbox: true, character_id, packages: [{
        id: packageId, quote_id: expect.any(String), title: 'E2E coin package', coins: 500,
        price: { minor_units: 499, currency: 'PLN', exponent: 2 }, valid_for_seconds: 300,
      }],
    });
    return body.packages[0].quote_id;
  };
  const labels = checkoutCopy[lang];
  let paidOrder;
  for (const [action, expectedState, character_id] of [['pay', 'paid', owned], ['fail', 'failed', null]]) {
    const quote = await catalogue(character_id);
    const data = { version: 1, request_id: uuid7(), package_id: packageId, quote_id: quote, character_id };
    const created = await post(data);
    expect(created.status()).toBe(200);
    const order = await created.json();
    expect(order).toMatchObject({ request_id: data.request_id, quote_id: quote, coins: 500, state: 'awaiting_payment' });
    const checkout = '/native-account/coin-checkout/' + order.order_id;
    if (action === 'pay') paidOrder = { ...order, checkout };
    expect(order.checkout_url).toBe(origin + checkout);
    await json(await post(data), 200, order);
    await json(await orderStatus(data.request_id), 200, order);
    await empty(await post({ ...data, quote_id: uuid7() }), 409, 'no-store');
    await empty(await post({ ...data, request_id: uuid7() }), 409, 'no-store');

    const guest = await request.get(checkout, { headers, maxRedirects: 0 });
    expect(guest.status()).toBe(303);
    expect(guest.headers().location).toBe(`/${lang}/login`);
    if (action === 'pay') {
      const context = await browser.newContext({
        baseURL: origin,

        locale: lang,
        viewport: info.project.use.viewport,
        extraHTTPHeaders: requestSource(info, 9),
      });
      try {
        const viewer = await context.newPage();
        await signIn(viewer, lang, other);
        await empty(await viewer.request.get(checkout), 404, 'no-store');
      } finally {
        await context.close();
      }
    }

    const hosted = await page.goto(checkout);
    expect(hosted.status()).toBe(200);
    expect(hosted.headers()['referrer-policy']).toBe('same-origin');
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    await expect(page).toHaveTitle(labels.title + ' · Oteryn');
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(labels.title);
    await expect(page.locator('main > p').first()).toHaveText(labels.note);
    await expect(page.locator('main dt')).toHaveText(labels.labels);
    await expect(page.locator('main dd')).toHaveText([order.order_id, '500', '4.99 PLN', labels.awaiting]);
    for (const decision of ['pay', 'fail']) {
      await expect(page.getByRole('button', { name: labels[decision], exact: true })).toBeVisible();
      await csrfRejected(page, checkout, { action: decision });
    }
    const csrf = await page.locator('input[name=_csrf]').first().inputValue();
    await empty(await page.request.post(checkout, {
      form: { _csrf: csrf, action: 'unknown' }, headers: { origin },
    }), 400, 'no-store');
    await submit(page, `main form:has([name=action][value=${action}])`);
    await expect(page.locator('main dd').last()).toHaveText(labels[expectedState]);
    await expect(page.locator('main > p').last()).toHaveText(labels.done);
    await expect(page.locator('main form')).toHaveCount(0);
    const status = await orderStatus(data.request_id);
    expect(status.status()).toBe(200);
    const result = await status.json();
    expect(result).toMatchObject({ order_id: order.order_id, state: expectedState, checkout_url: null });
    expect(Boolean(result.receipt_id)).toBe(action === 'pay');
    const replay = await page.request.post(checkout, {
      form: { _csrf: csrf, action }, headers: { origin }, maxRedirects: 0,
    });
    expect(replay.status()).toBe(303);
    expect(replay.headers().location).toBe(checkout);
    await json(await orderStatus(data.request_id), 200, result);
  }
  const paidRevision = await balance(500);
  expect(paidRevision).toBeGreaterThan(originalRevision);
  const reference = await providerReference(paidOrder.order_id);
  const badAmount = reversal('refund', reference, 498);
  await json(await providerEvent(request, JSON.stringify(badAmount)), 422, {
    version: 1, event_id: badAmount.event_id, code: 'PROVIDER_WEBHOOK_EVENT_REJECTED',
  });
  expect(await balance(500)).toBe(paidRevision);
  let lastRevision = paidRevision;
  for (const [type, state] of [['refund', 'refunded'], ['chargeback', 'chargeback']]) {
    const event = reversal(type, reference);
    const payload = JSON.stringify(event);
    const result = { version: 1, event_id: event.event_id, outcome: 'applied' };
    await json(await providerEvent(request, payload), 200, { ...result, replay: false });
    const revision = await balance(0);
    expect(revision).toBeGreaterThan(lastRevision);
    lastRevision = revision;
    await json(await providerEvent(request, payload), 200, { ...result, replay: true });
    expect(await balance(0)).toBe(revision);
    expect((await page.goto(paidOrder.checkout)).status()).toBe(200);
    await expect(page.locator('main dd').last()).toHaveText(labels[state]);
    const response = await orderStatus(paidOrder.request_id);
    expect(response.status()).toBe(200);
    expect((await response.json()).state).toBe(state);
  }

  const illegal = reversal('refund', reference);
  await json(await providerEvent(request, JSON.stringify(illegal)), 422, {
    version: 1, event_id: illegal.event_id, code: 'PROVIDER_WEBHOOK_ILLEGAL_TRANSITION',
  });

  const pending = await post({
    version: 1, request_id: uuid7(), package_id: packageId,
    quote_id: await catalogue(null), character_id: null,
  });
  expect(pending.status()).toBe(200);
  const pendingOrder = await pending.json();
  const earlyRefund = reversal('refund', await providerReference(pendingOrder.order_id));
  await json(await providerEvent(request, JSON.stringify(earlyRefund)), 200, {
    version: 1, event_id: earlyRefund.event_id, outcome: 'applied', replay: false,
  });
  expect((await page.goto(pendingOrder.checkout_url)).status()).toBe(200);
  await expect(page.locator('main dd').last()).toHaveText(labels.processing);
  await expect(page.locator('main form')).toHaveCount(0);
  expect(await balance(0, true)).toBeGreaterThan(lastRevision);
});
