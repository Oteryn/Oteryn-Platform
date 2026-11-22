import { spawnSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { crc32 } from 'node:zlib';
import { test, expect } from '../fixtures.mjs';
import { password, pool, submit } from '../helpers.mjs';
import { loginWithRecoveryCode, query } from '../staff-rbac-helpers.mjs';

const route = '/admin/media';
const form = `main form[enctype="multipart/form-data"][action^="${route}"]`;
const maxFileBytes = 8 * 1024 * 1024;
const file = (ext, mimeType, basename = 'editorial') => ({
  name: `${basename}.${ext}`,
  mimeType,
  buffer: readFileSync(new URL(`../fixtures/uploads/${basename}.${ext}`, import.meta.url)),
});
const png = file('png', 'image/png');
const mediaLabel = () => `Editorial upload ${randomUUID()}`;

async function staffAccount(browser, project, role) {
  const lang = project.metadata.locale;
  const account = { email: `upload-${randomUUID()}@example.invalid`, password: password() };
  // pool() validates the disposable loopback database before the seed CLI sees it.
  const db = pool();
  const target = process.env.CARGO_TARGET_DIR ??
    fileURLToPath(new URL('../../target', import.meta.url));
  const binary = process.env.PORTAL_E2E_BINARY ?? path.resolve(target, 'debug/oteryn-portal');
  function seed(entity, args) {
    const seeded = spawnSync(binary, ['seed', entity, JSON.stringify(args)], {
      env: { ...process.env, OTERYN_ENV: 'testing', OTERYN_DATABASE_URL: db.options.connectionString },
      encoding: 'utf8',
      timeout: 30_000,
    });
    if (seeded.status !== 0) throw new Error(`The upload fixture seed CLI failed for ${entity}`);
    return JSON.parse(seeded.stdout);
  }
  let accountId;
  let recoveryCode;
  try {
    ({ accountId } = seed('account', { ...account, locale: lang }));
    [recoveryCode] = seed('account-mfa', { accountId, password: account.password }).recoveryCodes;
    expect(typeof recoveryCode).toBe('string');
    const grant = await db.query(
      'INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2',
      [accountId, role],
    );
    expect(grant.rowCount).toBe(1);
  } finally {
    await db.end();
  }

  const { baseURL, ignoreHTTPSErrors, locale, viewport } = project.use;
  const context = await browser.newContext({ baseURL, ignoreHTTPSErrors, locale, viewport });
  try {
    const page = await context.newPage();
    await loginWithRecoveryCode(page, lang, account, recoveryCode);
    expect((await page.goto(`/admin?lang=${lang}`)).status()).toBe(200);
    return { accountId, state: await context.storageState() };
  } finally {
    await context.close();
  }
}

let editor;
test.beforeAll(async ({ browser }, { project }) => {
  editor = await staffAccount(browser, project, 'content_editor');
});

test.beforeEach(async ({ context }) => {
  await context.addCookies(editor.state.cookies);
});

async function openMedia(page, lang) {
  const response = await page.goto(`${route}?lang=${lang}`);
  expect(response.status()).toBe(200);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.locator(form).locator('button')).toHaveText(lang === 'pl' ? 'Prześlij' : 'Upload');
}

function postImage(page, image, alt, csrf) {
  return page.request.post(route, {
    multipart: { ...(csrf === undefined ? {} : { _csrf: csrf }), alt_text: alt, image },
    headers: { origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });
}

async function expectNoMedia(alt) {
  expect(await query('SELECT id FROM media WHERE alt_text=$1', [alt])).toEqual([]);
}

for (const { image, width, height, thumbnailHeight } of [
  { image: { ...png, name: 'Zażółć-gęślą-東京.png' }, width: 640, height: 160, thumbnailHeight: 80 },
  { image: file('jpeg', 'image/jpeg'), width: 640, height: 160, thumbnailHeight: 80 },
  { image: file('webp', 'image/webp'), width: 640, height: 160, thumbnailHeight: 80 },
  { image: file('jpeg', 'image/jpeg', 'progressive-444'), width: 4608, height: 3456, thumbnailHeight: 240 },
]) {
  test(`editor uploads, renders and retrieves ${image.mimeType} (${image.name})`, async ({ page }, { project }) => {
    const lang = project.metadata.locale;
    const alt = mediaLabel();
    await openMedia(page, lang);
    await page.locator(form).locator('input[name=image]').setInputFiles(image);
    const [uploaded] = await Promise.all([
      page.waitForResponse(response =>
        new URL(response.url()).pathname === route && response.request().method() === 'POST'),
      submit(page, form, { alt_text: alt }),
    ]);
    expect(uploaded.status()).toBe(302);
    expect(uploaded.headers().location).toBe(lang === 'en' ? `${route}?lang=en` : route);
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    const thumbnail = page.getByRole('img', { name: alt, exact: true });
    await expect(thumbnail).toBeVisible();
    await expect.poll(() => thumbnail.evaluate(img => [img.naturalWidth, img.naturalHeight])).toEqual([320, thumbnailHeight]);

    const rows = await query('SELECT id, mime_type, width, height FROM media WHERE alt_text=$1', [alt]);
    expect(rows).toEqual([{ id: expect.any(String), mime_type: image.mimeType, width, height }]);
    const media = `${route}/${rows[0].id}`;
    await expect(thumbnail).toHaveAttribute('src', `${media}/thumbnail`);
    for (const variant of ['content', 'thumbnail']) {
      const response = await page.request.get(`${media}/${variant}`, { maxRedirects: 0 });
      expect(response.status()).toBe(200);
      expect(response.headers()).toMatchObject({
        'content-type': image.mimeType,
        'x-content-type-options': 'nosniff',
        'content-disposition': 'inline',
      });
      expect((await response.body()).length).toBeGreaterThan(0);
    }
    const localeQuery = lang === 'en' ? '?lang=en' : '';
    await submit(page, `main form[action="${media}/delete${localeQuery}"]`);
    await expect(page).toHaveURL(`${route}${localeQuery}`);
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    await expect(thumbnail).toHaveCount(0);
    expect((await page.request.get(`${media}/content`)).status()).toBe(404);
  });
}

test('media pagination uses the selected language', async ({ page }, { project }) => {
  const lang = project.metadata.locale;
  const alt = mediaLabel();
  const hash = createHash('sha256').update(png.buffer).digest();
  const seeded = await query(
    `INSERT INTO media (id, alt_text, mime_type, width, height, content, sha256)
     SELECT uuidv7(), $1, 'image/png', 640, 160, $2, $3 FROM generate_series(1, 101)
     RETURNING id`,
    [alt, png.buffer, hash],
  );
  try {
    await openMedia(page, lang);
    await expect(page.locator('main ul > li')).toHaveCount(100);
    const next = page.getByRole('link', { name: lang === 'en' ? 'Next' : 'Następna', exact: true });
    await expect(next).toHaveAttribute('href', `?page=1&lang=${lang}`);
    await next.click();
    await expect(page).toHaveURL(`${route}?page=1&lang=${lang}`);
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    await expect(page.getByRole('img', { name: alt, exact: true })).toHaveCount(1);
  } finally {
    await query('DELETE FROM media WHERE id = ANY($1::uuid[])', [seeded.map(row => row.id)]);
  }
});

function pngOfSize(fileBytes) {
  // A valid text chunk before IEND lets us control upload size without changing the pixels.
  const dataLength = fileBytes - png.buffer.length - 12;
  const chunk = Buffer.alloc(dataLength + 12, 'a');
  chunk.writeUInt32BE(dataLength);
  chunk.write('tEXtComment\0', 4, 'ascii');
  chunk.writeUInt32BE(crc32(chunk.subarray(4, -4)), chunk.length - 4);
  return { ...png, buffer: Buffer.concat([png.buffer.subarray(0, -12), chunk, png.buffer.subarray(-12)]) };
}

test('signed-in editor submits a 7 MiB native form over a 10 Mbps uplink', async ({ page, context }, { project }) => {
  const lang = project.metadata.locale;
  const alt = mediaLabel();
  await openMedia(page, lang);
  const image = pngOfSize(7 * 1024 * 1024);
  await page.locator(form).locator('input[name=image]').setInputFiles(image);
  await page.locator(form).locator('[name=alt_text]').fill(alt);

  const cdp = await context.newCDPSession(page);
  try {
    await cdp.send('Network.emulateNetworkConditions', {
      offline: false,
      latency: 0,
      downloadThroughput: -1,
      uploadThroughput: 1_250_000,
    });
    const started = performance.now();
    const [uploaded] = await Promise.all([
      page.waitForResponse(response =>
        new URL(response.url()).pathname === route && response.request().method() === 'POST'),
      page.locator(form).locator('button').click(),
    ]);
    expect(await uploaded.request().headerValue('x-csrf-token')).toBeNull();
    expect(uploaded.status()).toBe(302);
    expect(performance.now() - started).toBeGreaterThan(5000);
    const localeQuery = lang === 'en' ? '?lang=en' : '';
    await expect(page).toHaveURL(`${route}${localeQuery}`);
    await expect(page.getByRole('img', { name: alt, exact: true })).toBeVisible();
    const rows = await query('SELECT id, mime_type FROM media WHERE alt_text=$1', [alt]);
    expect(rows).toEqual([{ id: expect.any(String), mime_type: 'image/png' }]);
    await submit(page, `main form[action="${route}/${rows[0].id}/delete${localeQuery}"]`);
    await expect(page.getByRole('img', { name: alt, exact: true })).toHaveCount(0);
  } finally {
    await cdp.detach();
  }
});

const rejectedImages = [
  ['file greater than 8 MiB', () => pngOfSize(maxFileBytes + 1), 422],
  ['multipart body above the request limit', () => ({ ...png, buffer: Buffer.alloc(maxFileBytes + 16385) }), 413],
  ['declared MIME does not match the PNG', () => ({ ...png, mimeType: 'image/jpeg' }), 422],
  ['extension does not match the PNG', () => ({ ...png, name: 'editorial.jpeg' }), 422],
  ['SVG with an empty script element', () => ({
    name: 'unsupported.svg',
    mimeType: 'image/svg+xml',
    buffer: readFileSync(new URL('../fixtures/uploads/unsupported.svg', import.meta.url)),
  }), 422],
  ['PNG with extra bytes after the image container', () => ({
    ...png,
    buffer: Buffer.concat([png.buffer, Buffer.from('end of fixture')]),
  }), 422],
  ['PNG with truncated pixel data', () => file('png', 'image/png', 'truncated-pixels'), 422],
  ['JPEG with a truncated scan', () => file('jpeg', 'image/jpeg', 'truncated-scan'), 422],
  ['zero-byte PNG', () => ({ ...png, buffer: Buffer.alloc(0) }), 422],
];

for (const [label, fixture, status] of rejectedImages) {
  test(`editorial upload rejects ${label}`, async ({ page }, { project }) => {
    const alt = mediaLabel();
    await openMedia(page, project.metadata.locale);
    const csrf = await page.locator(form).locator('input[name=_csrf]').inputValue();
    expect((await postImage(page, fixture(), alt, csrf)).status()).toBe(status);
    await expectNoMedia(alt);
  });
}

test('editorial upload requires the editor’s CSRF token', async ({ page }, { project }) => {
  const alt = mediaLabel();
  await openMedia(page, project.metadata.locale);
  expect((await postImage(page, png, alt)).status()).toBe(403);
  await expectNoMedia(alt);
});

test('MFA-authenticated support staff cannot upload editorial media', async ({ browser }, { project }) => {
  const support = await staffAccount(browser, project, 'support');
  const { baseURL, ignoreHTTPSErrors, locale, viewport } = project.use;
  const context = await browser.newContext({ baseURL, ignoreHTTPSErrors, locale, viewport, storageState: support.state });
  try {
    const page = await context.newPage();
    const alt = mediaLabel();
    expect((await page.goto(`/${locale}/`)).status()).toBe(200);
    const csrf = await page.locator('form[action$="/logout"] input[name=_csrf]').first().inputValue();
    expect((await postImage(page, png, alt, csrf)).status()).toBe(403);
    await expectNoMedia(alt);
    expect(await query(
      "SELECT target FROM audit_log WHERE actor=$1 AND action='admin.access.denied' AND target=$2",
      [support.accountId, route],
    )).toEqual([{ target: route }]);
  } finally {
    await context.close();
  }
});
