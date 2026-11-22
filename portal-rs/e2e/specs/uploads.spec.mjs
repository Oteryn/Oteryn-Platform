import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
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
const file = (ext, mimeType) => ({
  name: `editorial.${ext}`,
  mimeType,
  buffer: readFileSync(new URL(`../fixtures/uploads/editorial.${ext}`, import.meta.url)),
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

for (const image of [
  { ...png, name: 'Zażółć-gęślą-東京.png' },
  file('jpeg', 'image/jpeg'),
  file('webp', 'image/webp'),
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
    await expect.poll(() => thumbnail.evaluate(img => [img.naturalWidth, img.naturalHeight])).toEqual([320, 80]);

    const rows = await query('SELECT id, mime_type, width, height FROM media WHERE alt_text=$1', [alt]);
    expect(rows).toEqual([{ id: expect.any(String), mime_type: image.mimeType, width: 640, height: 160 }]);
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
    await submit(page, `main form[action="${media}/delete"]`);
    await expect(thumbnail).toHaveCount(0);
    expect((await page.request.get(`${media}/content`)).status()).toBe(404);
  });
}

function oversizedPng() {
  // Add a valid text chunk before IEND so the only violation is the file size.
  const dataLength = maxFileBytes + 1 - png.buffer.length - 12;
  const chunk = Buffer.alloc(dataLength + 12, 'a');
  chunk.writeUInt32BE(dataLength);
  chunk.write('tEXtComment\0', 4, 'ascii');
  chunk.writeUInt32BE(crc32(chunk.subarray(4, -4)), chunk.length - 4);
  return { ...png, buffer: Buffer.concat([png.buffer.subarray(0, -12), chunk, png.buffer.subarray(-12)]) };
}

const rejectedImages = [
  ['file greater than 8 MiB', oversizedPng, 422],
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
