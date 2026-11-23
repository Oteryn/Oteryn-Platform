import { randomBytes } from 'node:crypto';
import { waitForLoginBudget } from '../helpers.mjs';
import { digest, expect, formPost, text } from './fixtures.mjs';

export async function authorize(page, lang, scope = 'game:ticket', clientId = 'e2e-local') {
  const verifier = randomBytes(32).toString('base64url');
  const redirect = 'https://localhost:18443/__oauth/callback';
  const state = randomBytes(24).toString('hex');
  const params = {
    response_type: 'code', client_id: clientId, redirect_uri: redirect, scope, state,
    code_challenge: digest(verifier).toString('base64url'), code_challenge_method: 'S256',
  };
  // The authorize GET, consent POST and immediate token exchange use the
  // same source bucket as browser login.
  await waitForLoginBudget(3);
  expect((await page.goto(`/oauth/authorize?${new URLSearchParams(params)}`)).status()).toBe(200);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  const approve = 'main form[action="/oauth/authorize"]';
  await expect(page.locator(`${approve} button[value=approve]`)).toHaveText(
    text(lang, 'Allow', 'Zezwól'));
  const response = await formPost(page, approve, { decision: 'approve' });
  expect(response.status()).toBe(302);
  const callback = new URL(response.headers().location);
  expect(callback.origin + callback.pathname).toBe(redirect);
  expect(callback.searchParams.get('state') === state, 'callback state matches').toBe(true);
  const code = callback.searchParams.get('code');
  expect(Boolean(code), 'callback includes authorization code').toBe(true);
  return { grant_type: 'authorization_code', client_id: params.client_id, redirect_uri: redirect,
    code, code_verifier: verifier };
}

export async function token(page, form) {
  const response = await page.request.post('/oauth/token', { form, maxRedirects: 0 });
  expect(response.headers()['cache-control']).toBe('no-store');
  return response;
}

export async function tokens(page, form) {
  const response = await token(page, form);
  expect(response.status()).toBe(200);
  const issued = await response.json();
  expect(Boolean(issued.access_token && issued.refresh_token), 'issued both tokens').toBe(true);
  return issued;
}
