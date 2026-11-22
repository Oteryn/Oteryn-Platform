import { createHash, randomBytes } from 'node:crypto';
import { expect, test } from '@playwright/test';
import { login, pool } from '../helpers.mjs';
import { qualifyRememberedDevice } from './device-flow.mjs';
import { connect, exchange, fixture, harness, jsonBytes, wire, withUnknown } from './transport.mjs';

async function oauthBearer(page, locale, email, password) {
  await login(page, locale, { email, password });
  const verifier = randomBytes(32).toString('base64url');
  const state = randomBytes(24).toString('hex');
  const clientId = '019a5f4a-1000-7000-8000-000000000001';
  const redirect = 'http://127.0.0.1:41000/callback';
  const query = new URLSearchParams({
    response_type: 'code', client_id: clientId, redirect_uri: redirect, scope: 'game:ticket', state,
    code_challenge: createHash('sha256').update(verifier).digest('base64url'), code_challenge_method: 'S256',
  });
  const consent = await page.goto(`/oauth/authorize?${query}`);
  expect(consent.status()).toBe(200);
  const form = await page.locator('main form').first().evaluate(element => Object.fromEntries(new FormData(element)));
  const origin = new URL(page.url()).origin;
  let bearer;
  try {
    const approval = await page.request.post('/oauth/authorize', {
      form: { ...form, decision: 'approve' }, headers: { origin }, maxRedirects: 0,
    });
    expect(approval.status()).toBe(302);
    const callback = new URL(approval.headers().location);
    expect(callback.origin + callback.pathname).toBe(redirect);
    expect(callback.searchParams.get('state') === state).toBe(true);
    const token = await page.request.post('/oauth/token', { form: {
      grant_type: 'authorization_code', client_id: clientId, redirect_uri: redirect,
      code: callback.searchParams.get('code'), code_verifier: verifier,
    } });
    expect(token.status()).toBe(200);
    bearer = (await token.json()).access_token;
  } catch {
    throw new Error('The local OAuth consent or token exchange failed');
  }
  expect(/^[A-Za-z0-9_-]{43}$/.test(bearer)).toBe(true);
  return 'Bearer ' + bearer;
}

async function nextProjectionEpoch() {
  // One worker publishes to the shared harness projection. Read the high-water
  // mark so additional projects and repeat-each invocations advance it.
  const db = pool();
  try {
    const { rows: [feed] } = await db.query('SELECT highest_epoch FROM account_character_feed WHERE id = 1');
    return String(BigInt(feed.highest_epoch ?? '0') + 1n);
  } finally {
    await db.end();
  }
}

test('an OAuth PKCE bearer can issue one native ticket and has a separate character budget', async ({ page, browser }, info) => {
  const locale = info.project.metadata.locale;
  const auth = await oauthBearer(page, locale, 'coin-1@example.invalid', process.env.PORTAL_API_LOGIN_PASSWORD);
  const other = await browser.newContext({ baseURL: process.env.PORTAL_E2E_ORIGIN,
    locale, viewport: info.project.use.viewport, ignoreHTTPSErrors: true });
  try {
    const otherPage = await other.newPage();
    const otherAuth = await oauthBearer(otherPage, locale, 'coin-2@example.invalid', process.env.PORTAL_API_OTHER_LOGIN_PASSWORD);
    await qualifyNativeRoutes(page, auth, otherPage.request, otherAuth);
  } finally {
    await other.close();
  }
});

async function qualifyNativeRoutes(page, auth, otherRequest, otherAuth) {
  const tickets = '/api/v1/game-auth/tickets';
  const characters = '/api/v1/game-auth/native-characters';
  const request = fixture('native-oauth-ticket-v1', 'ticket-request.json');

  const responseBytes = fixture('native-characters-v1', 'characters-response.json');
  const projection = await connect('internal', 'lcfa');
  const wrongPeer = await connect('internal', 'runtime-status');
  const ingest = '/internal/v1/game-auth/native-account-characters';
  const source = 'oteryn-game-e2e';
  const epoch = await nextProjectionEpoch();
  const timestamp = String(Math.floor(Date.now() / 1000));
  const snapshot = jsonBytes({
    contract_version: 1, operation: 'PublishAccountCharactersV1', source_authority: source,
    account_id: harness().seed.coinAccount1.accountId, projection_epoch: epoch, projection_revision: '1',
    source_observed_at: timestamp, characters: JSON.parse(responseBytes).characters,
  });
  const watermark = jsonBytes({
    contract_version: 1, operation: 'PublishProjectionWatermarkV1', source_authority: source,
    projection_epoch: epoch, complete_through: timestamp, observed_at: timestamp,
  });
  const accepted = '{"contract_version":1,"result":"accepted"}';
  try {
    await wire(await exchange(wrongPeer, 'POST', ingest, snapshot), 401, '');
    await wire(await exchange(wrongPeer, 'POST', ingest + '/watermark', watermark), 401, '');
    await wire(await exchange(projection, 'POST', ingest, withUnknown(snapshot)), 400, '');
    await wire(await exchange(projection, 'POST', ingest + '/watermark', withUnknown(watermark)), 400, '');
    await wire(await exchange(projection, 'POST', ingest, snapshot), 200, accepted);
    await wire(await exchange(projection, 'POST', ingest, snapshot), 200, accepted);
    await wire(await exchange(page.request, 'GET', characters, undefined, auth), 503, '');
    await wire(await exchange(projection, 'POST', ingest + '/watermark', watermark), 200, accepted);
    await wire(await exchange(projection, 'POST', ingest + '/watermark', watermark), 200,
      '{"contract_version":1,"result":"superseded"}');
  } finally {
    await Promise.all([projection.dispose(), wrongPeer.dispose()]);
  }
  await wire(await exchange(otherRequest, 'GET', characters, undefined, otherAuth),
    200, fixture('native-characters-v1', 'characters-empty.json'));
  for (let attempt = 0; attempt < 9; attempt++) {
    await wire(await exchange(page.request, 'GET', characters, undefined, auth), 200, responseBytes);
  }
  const characterLimit = await exchange(page.request, 'GET', characters, undefined, auth);
  await wire(characterLimit, 429, '');
  expect(characterLimit.headers()['retry-after']).toBe('30');
  await qualifyRememberedDevice(auth, JSON.parse(responseBytes).characters);
  for (let attempt = 0; attempt < 4; attempt++) {
    await wire(await exchange(page.request, 'POST', tickets, withUnknown(request), auth),
      422, fixture('native-oauth-ticket-v1', 'ticket-invalid-request.json'));
  }
  const ticket = await exchange(page.request, 'POST', tickets, request, auth);
  expect(ticket.status()).toBe(200);
  expect(ticket.headers()['content-type']).toBe('application/json');
  expect(ticket.headers()['cache-control']).toContain('no-store');
  const issued = await ticket.text();
  const value = JSON.parse(issued).ticket;
  expect(/^[A-Za-z0-9_-]{43}$/.test(value) && Buffer.from(value, 'base64url').toString('base64url') === value).toBe(true);
  const expected = fixture('native-oauth-ticket-v1', 'ticket-response.json').toString();
  expect(issued.replace(value, JSON.parse(expected).ticket) === expected).toBe(true);

  await wire(await exchange(page.request, 'POST', tickets, request, auth),
    401, fixture('native-oauth-ticket-v1', 'ticket-unauthenticated.json'));
}
