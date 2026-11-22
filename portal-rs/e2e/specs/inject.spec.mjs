import { createHash, randomBytes } from 'node:crypto';
import { crc32 } from 'node:zlib';
import { test, expect } from '../fixtures.mjs';
import { login, register, submit, waitForWikiBudget } from '../helpers.mjs';
import { enrollMfa, loginWithRecoveryCode, query } from '../staff-rbac-helpers.mjs';

function expectDocumentPolicy(response, cacheControl) {
  const headers = response.headers();
  expect(headers['content-type']).toMatch(/^text\/html\b/);
  expect(headers['cache-control']).toBe(cacheControl);
  expect(headers['strict-transport-security']).toBe('max-age=63072000; includeSubDomains');
  expect(headers['x-content-type-options']).toBe('nosniff');
  expect(headers['referrer-policy']).toBe('strict-origin-when-cross-origin');
  expect(headers['permissions-policy']).toBe('camera=(), microphone=(), geolocation=(), payment=(), usb=()');
  expect(headers['cross-origin-opener-policy']).toBe('same-origin');

  const entries = (headers['content-security-policy'] ?? '').split(';').map(directive => {
    const [name, ...sources] = directive.trim().split(/\s+/);
    return [name, sources];
  });
  const directives = Object.fromEntries(entries);
  expect(Object.keys(directives)).toHaveLength(entries.length);
  return directives;
}

function expectShellPolicy(directives) {
  expect(directives).toEqual({
    'default-src': ["'none'"],
    'script-src': ["'self'", expect.stringMatching(/^'nonce-[a-f0-9]{64}'$/)],
    'style-src': ["'self'"],
    'img-src': ["'self'", 'data:'],
    'font-src': ["'self'"],
    'connect-src': ["'self'"],
    'form-action': ["'self'"],
    'object-src': ["'none'"],
    'base-uri': ["'none'"],
    'frame-ancestors': ["'none'"],
  });
  return directives['script-src'][1];
}

test('localized home and error documents enforce the shell policy with fresh script nonces', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const nonces = new Set();
  for (const [path, status, cacheControl] of [
    [`/${lang}/`, 200, 'no-store, max-age=0'],
    [`/${lang}/security-fixture-missing-page`, 404, 'no-store'],
    [`/${lang}/`, 200, 'no-store, max-age=0'],
  ]) {
    const response = await page.goto(path);
    expect(response.status(), path).toBe(status);
    expect(response.headers()['content-language']).toBe(lang);
    const nonce = expectShellPolicy(expectDocumentPolicy(response, cacheControl));
    expect(nonces.has(nonce), 'nonce reuse across documents').toBe(false);
    nonces.add(nonce);
  }
});

test('login uses its private cache and document policies', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const response = await page.goto(`/${lang}/login`);
  expect(response.status()).toBe(200);
  expect(response.headers()['content-language']).toBe(lang);
  expect(expectDocumentPolicy(response, 'no-store, private')).toEqual({
    'default-src': ["'self'"],
    'object-src': ["'none'"],
    'base-uri': ["'none'"],
    'frame-ancestors': ["'none'"],
    'font-src': ["'self'"],
    'img-src': ["'self'"],
  });
});

test('locale negotiation redirects to the relative localized homepage', async ({ page, baseURL }, info) => {
  const lang = info.project.metadata.locale;
  const response = await page.request.get('/', {
    headers: { 'accept-language': lang },
    maxRedirects: 0,
  });
  expect(response.status()).toBe(303);
  expect(response.headers().location).toBe(`/${lang}/`);
  expect((await page.goto('/')).status()).toBe(200);
  await expect(page).toHaveURL(new URL(`/${lang}/`, baseURL).href);
});

test('OAuth authorization rejects an unregistered callback without redirecting', async ({ page }) => {
  const verifier = randomBytes(32).toString('base64url');
  const authorization = new URLSearchParams({
    response_type: 'code',
    client_id: 'e2e-local',
    redirect_uri: 'https://localhost:18443/__oauth/callback',
    scope: 'game:ticket',
    state: randomBytes(24).toString('hex'),
    code_challenge: createHash('sha256').update(verifier).digest('base64url'),
    code_challenge_method: 'S256',
  });
  const registered = await page.request.get(`/oauth/authorize?${authorization}`, { maxRedirects: 0 });
  expect(registered.status()).toBe(303);
  expect(registered.headers().location).toMatch(/^\/(pl|en)\/login$/);

  authorization.set('redirect_uri', 'https://callback.example.invalid/oauth');
  const rejected = await page.request.get(`/oauth/authorize?${authorization}`, { maxRedirects: 0 });
  expect(rejected.status()).toBe(400);
  expect(rejected.headers().location).toBeUndefined();
});

test('search reflects ordinary punctuation as a single input value', async ({ page }, info) => {
  const route = `/${info.project.metadata.locale}/search`;
  await waitForWikiBudget(route, 2);
  expect((await page.goto(route)).status()).toBe(200);
  const form = page.locator('main form[method=get]').first();
  const query = `Label "A & B" < C, D's 10%_rate`;
  await form.locator('input[name=q]').fill(query);
  const [response] = await Promise.all([
    page.waitForResponse(candidate => candidate.request().isNavigationRequest() && new URL(candidate.url()).pathname === route),
    form.getByRole('button').click(),
  ]);
  expect(response.status()).toBe(200);
  expect(new URL(response.url()).searchParams.get('q')).toBe(query);
  await expect(page.locator('main input[name=q]')).toHaveValue(query);
  expect(await response.text()).toMatch(/&(?:quot|#0*34|#x0*22);A &(?:amp|#0*38|#x0*26); B&(?:quot|#0*34|#x0*22); &(?:lt|#0*60|#x0*3c); C/);
});

test('support tickets and reports preserve formatting examples as plain text', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  await login(page, lang, await register(page, lang));

  const ticket = `/${lang}/support/tickets`;
  expect((await page.goto(`${ticket}/create`)).status()).toBe(200);
  const subject = 'Label <em>subject</em> & details';
  const body = 'Formatting example <em>body</em> & details.';
  const form = page.locator('main form');
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  const token = await form.locator('input[name=_csrf]').inputValue();
  const mismatchedToken = `${token[0] === '0' ? '1' : '0'}${token.slice(1)}`;
  const origin = new URL(page.url()).origin;
  const ticketExists = async () => (await query(
    'SELECT EXISTS (SELECT 1 FROM tickets WHERE request_key = $1) AS present',
    [fields.request_key],
  ))[0].present;
  expect(await ticketExists()).toBe(false);
  for (const [label, csrf, requestOrigin] of [
    ['mismatched token', mismatchedToken, origin],
    ['untrusted origin', token, 'https://origin.example.invalid'],
  ]) {
    const response = await page.request.post(ticket, {
      form: { ...fields, subject, body, _csrf: csrf },
      headers: { origin: requestOrigin },
      maxRedirects: 0,
    });
    expect(response.status(), label).toBe(403);
    expect(await ticketExists(), label).toBe(false);
  }
  await submit(page, 'main form', { subject, body });
  await expect(page).toHaveURL(new RegExp(`${ticket}/[a-f0-9-]+$`));
  expect(await ticketExists()).toBe(true);
  await expect(page.locator('main h2')).toHaveText(subject);
  await expect(page.locator('main article p')).toHaveText(body);
  const reply = 'Follow-up <em>reply</em> & details.';
  await submit(page, 'main form[action$="/reply"]', { body: reply });
  await expect(page.locator('main article p')).toHaveText([body, reply]);
  await expect(page.locator('main article em')).toHaveCount(0);

  const reports = `/${lang}/support/reports`;
  expect((await page.goto(`${reports}/create`)).status()).toBe(200);
  const target = 'Example <em>target</em> & details';
  const evidence = 'Example <em>evidence</em> & details.';
  await submit(page, 'main form', { target_reference: target, evidence_summary: evidence });
  await expect(page).toHaveURL(new RegExp(`${reports}/[a-f0-9-]+$`));
  await expect(page.locator('main article').getByText(target, { exact: true })).toBeVisible();
  await expect(page.locator('main article').getByText(evidence, { exact: true })).toBeVisible();
  await expect(page.locator('main article em')).toHaveCount(0);
});

const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAHklEQVR4nGNQzrP5T03MMGrgqIGjBo4aOGrgSDUQANpkzt/P0xIDAAAAAElFTkSuQmCC', 'base64');

// PNG permits private ancillary chunks. Padding via a valid chunk keeps the
// image byte signature and decoder valid on both sides of the file-size limit.
function pngAtSize(size) {
  const chunk = Buffer.alloc(size - png.length);
  chunk.writeUInt32BE(chunk.length - 12);
  chunk.write('npAd', 4, 'ascii');
  chunk.writeUInt32BE(crc32(chunk.subarray(4, -4)), chunk.length - 4);
  return Buffer.concat([png.subarray(0, -12), chunk, png.subarray(-12)]);
}

test('editorial media validates image format and size and serves safe image headers', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await query(
    "INSERT INTO account_roles(account_id, role_id) SELECT a.id, r.id FROM accounts a CROSS JOIN roles r WHERE a.email = $1 AND r.key = 'platform_admin'",
    [account.email],
  );
  await login(page, lang, account);
  await enrollMfa(page, lang, account);
  const recoveryCode = await page.locator('main textarea[readonly]').first().inputValue();
  await loginWithRecoveryCode(page, lang, account, recoveryCode);

  const route = '/admin/media';
  expect((await page.goto(route)).status()).toBe(200);
  const form = page.locator('main form[action="/admin/media"]');
  const input = form.locator('input[type=file]');
  const alt = 'Diagram "A" with <em>label</em> & detail';
  const origin = new URL(page.url()).origin;
  const maxFileSize = 8 * 1024 * 1024;
  for (const [label, name, mimeType, buffer, status] of [
    ['unsupported extension', 'example.txt', 'text/plain', png, 422],
    ['incorrect content', 'example.png', 'image/png', Buffer.from('Plain text'), 422],
    ['file one byte over 8 MiB', 'example.png', 'image/png', pngAtSize(maxFileSize + 1), 422],
  ]) {
    const response = await page.request.post(route, {
      headers: { origin },
      multipart: {
        _csrf: await form.locator('input[name=_csrf]').inputValue(),
        alt_text: alt,
        image: { name, mimeType, buffer },
      },
      maxRedirects: 0,
    });
    expect(response.status(), label).toBe(status);
    expect((await page.goto(route)).status()).toBe(200);
    await expect(page.getByRole('img', { name: alt, exact: true })).toHaveCount(0);
  }

  await input.setInputFiles({ name: 'example.png', mimeType: 'image/png', buffer: pngAtSize(maxFileSize) });
  await submit(page, 'main form[action="/admin/media"]', { alt_text: alt });
  const image = page.getByRole('img', { name: alt, exact: true });
  await expect(image).toBeVisible();
  await expect(image).toHaveAttribute('alt', alt);
  expect(await image.evaluate(element => element.getAttributeNames().sort())).toEqual(['alt', 'src', 'width']);
  const caption = image.locator('..').locator(':scope > p');
  await expect(caption).toBeVisible();
  expect(await caption.textContent()).toBe(alt);
  expect(await caption.evaluate(element => element.getAttributeNames())).toEqual([]);
  await expect(caption.locator('*')).toHaveCount(0);
  const thumbnail = await image.getAttribute('src');
  expect(thumbnail).toMatch(/^\/admin\/media\/[a-f0-9-]+\/thumbnail$/);
  const content = thumbnail.replace(/\/thumbnail$/, '/content');
  for (const path of [thumbnail, content]) {
    const response = await page.request.get(path);
    expect(response.status()).toBe(200);
    const headers = response.headers();
    expect(headers['content-type']).toBe('image/png');
    expect(headers['x-content-type-options']).toBe('nosniff');
    expect(headers['content-disposition']).toBe('inline');
    expect(headers['cache-control']).toMatch(/(?:^|,\s*)no-store(?:,|$)/);
    expect((await response.body()).subarray(0, 8)).toEqual(png.subarray(0, 8));
  }
  await submit(page, `main form[action="${thumbnail.replace(/\/thumbnail$/, '/delete')}"]`);
  expect((await page.request.get(content)).status()).toBe(404);
});
