import { execFile, spawn } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { once } from 'node:events';
import { readFileSync, readdirSync } from 'node:fs';
import http from 'node:http';
import https from 'node:https';
import net from 'node:net';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import pg from 'pg';
import { test, expect } from '../fixtures.mjs';
import { messages } from '../public-depth-helpers.mjs';
import { seededStaff } from '../staff-rbac-helpers.mjs';

const run = promisify(execFile);
const fixture = fileURLToPath(new URL('../../crates/downloads/tests/fixtures/origin-v1/stable/', import.meta.url));
const policy = path.join(fixture, 'policy-v1.json');
const metadata = readdirSync(path.join(fixture, 'metadata'))
  .map(name => path.join(fixture, 'metadata', name));
const downloads = '/admin/downloads';

async function listen(server) {
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  return server.address().port;
}

// Use a fresh local database per project so each publishes the same offline-signed
// fixture once. Inject its public root through the normal operator configuration.
async function withReleasePortal(work) {
  const url = new URL(process.env.PORTAL_E2E_ADMIN_DATABASE_URL);
  expect(['localhost', '127.0.0.1', '[::1]']).toContain(url.hostname);
  const admin = new pg.Client({ connectionString: url.href });
  const name = 'portal_e2e_gap54_' + randomBytes(6).toString('hex');
  const previous = process.env.PORTAL_E2E_DATABASE_URL;
  let created = false;
  let proxy;
  let server;
  try {
    await admin.connect();
    await admin.query(`CREATE DATABASE "${name}"`);
    created = true;
    url.pathname = '/' + name;
    process.env.PORTAL_E2E_DATABASE_URL = url.href;

    const reservation = net.createServer();
    const backend = await listen(reservation);
    await promisify(reservation.close.bind(reservation))();
    const scratch = path.dirname(process.env.SSL_CERT_FILE);
    proxy = https.createServer({
      key: readFileSync(path.join(scratch, 'key.pem')),
      cert: readFileSync(path.join(scratch, 'cert.pem')),
    }, (request, response) => {
      const upstream = http.request({
        hostname: '127.0.0.1', port: backend, path: request.url,
        method: request.method, headers: request.headers,
      }, reply => {
        response.writeHead(reply.statusCode, reply.headers);
        reply.pipe(response);
      });
      upstream.on('error', () => { response.writeHead(502); response.end(); });
      request.pipe(upstream);
    });
    const port = await listen(proxy);
    const origin = `https://localhost:${port}`;
    const env = {
      OTERYN_ENV: 'testing',
      OTERYN_DATABASE_URL: url.href,
      OTERYN_BIND: `127.0.0.1:${backend}`,
      OTERYN_ADMIN_PANEL: 'true',
      OTERYN_ADMIN_HOST: `localhost:${port}`,
      OTERYN_PUBLIC_ORIGIN: origin,
      OTERYN_ACCOUNT_LOGIN: 'true',
      OTERYN_ACCOUNT_MFA: 'true',
      OTERYN_ACCOUNT_CREDENTIALS: 'true',
      OTERYN_MFA_KEY: process.env.OTERYN_MFA_KEY,
      OTERYN_NOTIFY_UNSUBSCRIBE_KEY: process.env.OTERYN_NOTIFY_UNSUBSCRIBE_KEY,
      OTERYN_NOTIFY_UNSUBSCRIBE_URL: `${origin}/en/unsubscribe`,
      OTERYN_STATIC_DIR: process.env.OTERYN_STATIC_DIR,
      OTERYN_DOWNLOADS_ENABLED: 'true',
      OTERYN_DOWNLOADS_ROOT_STABLE: path.join(fixture, 'metadata', '1.root.json'),
      OTERYN_WORLD_ADMIN: 'true',
      PORTAL_NATIVE_ADMISSION_BIND: '127.0.0.1:0',
    };
    for (const suffix of ['FILE', 'SHA256', 'SOURCE_REVISION', 'AUTHORITY_EPOCH',
      'RULESET_ID', 'PROFILE_ID', 'SNAPSHOT_ID']) {
      const key = 'OTERYN_WORLD_CATALOG_' + suffix;
      if (process.env[key]) env[key] = process.env[key];
    }
    const binary = process.env.PORTAL_E2E_BINARY;
    await run(binary, ['migrate'], { env }).catch(error => {
      throw new Error(`Release test migrations failed (exit ${error.code})`);
    });
    server = spawn(binary, ['serve'], { env, stdio: 'ignore' });
    await expect.poll(async () => (await fetch(`http://127.0.0.1:${backend}/readyz`)
      .catch(() => null))?.status, { timeout: 20_000 }).toBe(200);
    expect(env.OTERYN_WORLD_CATALOG_SNAPSHOT_ID, 'local catalog fixture').toMatch(/^sha256:[a-f0-9]{64}$/);
    await work(origin, env.OTERYN_WORLD_CATALOG_SNAPSHOT_ID);
  } finally {
    if (server && server.exitCode === null && server.signalCode === null) {
      const stopped = once(server, 'exit');
      server.kill('SIGINT');
      await stopped;
    }
    if (proxy) {
      proxy.closeAllConnections();
      await promisify(proxy.close.bind(proxy))();
    }
    if (previous === undefined) delete process.env.PORTAL_E2E_DATABASE_URL;
    else process.env.PORTAL_E2E_DATABASE_URL = previous;
    try {
      if (created) await admin.query(`DROP DATABASE "${name}" WITH (FORCE)`);
    } finally {
      await admin.end();
    }
  }
}

test('operators publish signed releases and inspect publication and catalog status', async ({ browser }, info) => {
  const lang = info.project.metadata.locale;
  const labels = messages('templates/admin', lang);
  const catalog = messages('templates/admin/world', lang);
  await withReleasePortal(async (origin, snapshot) => {
    const context = await browser.newContext({
      baseURL: origin, locale: lang, viewport: info.project.use.viewport, ignoreHTTPSErrors: true,
    });
    try {
      const page = await context.newPage();
      const staff = await seededStaff(page, lang, 'platform_admin');
      const home = new URL('/admin', origin).href;
      expect((await page.goto(home)).status()).toBe(200);
      await page.getByRole('link', { name: labels['downloads-title'], exact: true }).click();
      await expect(page.locator('html')).toHaveAttribute('lang', lang);
      const form = page.locator('#signed-release');
      await expect(form.locator('select')).toHaveValue('stable');
      const csrf = await form.locator('input[name=_csrf]').inputValue();
      const target = new URL(downloads, origin).href;
      const payload = {
        channel: 'stable',
        policy: readFileSync(policy).toString('base64'),
        metadata: metadata.map(file => ({ name: path.basename(file), content: readFileSync(file).toString('base64') })),
      };
      expect((await page.request.post(target, { data: payload, headers: { origin } })).status()).toBe(403);

      const otherContext = await browser.newContext({
        baseURL: origin, locale: lang, viewport: info.project.use.viewport, ignoreHTTPSErrors: true,
      });
      try {
        const other = await otherContext.newPage();
        await seededStaff(other, lang, 'moderator');
        expect((await other.goto(home)).status()).toBe(200);
        await expect(other.locator('main nav a[href="/admin/downloads"]')).toHaveCount(0);
        expect((await other.request.get(target)).status()).toBe(403);
        // Use a token issued to this signed-in staff account for the permission check.
        const loginForm = await other.goto(`${origin}/${lang}/account/security`);
        expect(loginForm.status()).toBe(200);
        const otherToken = await other.locator('input[name=_csrf]').first().inputValue();
        expect((await other.request.post(target, {
          data: payload, headers: { origin, 'x-csrf-token': otherToken },
        })).status()).toBe(403);
      } finally {
        await otherContext.close();
      }

      const button = form.getByRole('button', { name: labels['downloads-publish'] });
      const policyInput = form.locator('input[name=policy]');
      const metadataInput = form.locator('input[name=metadata]');
      await policyInput.setInputFiles({ name: 'policy-v1.json', mimeType: 'application/json', buffer: Buffer.from('{') });
      await metadataInput.setInputFiles(metadata);
      await button.click();
      await expect(form.getByRole('alert')).toHaveText(labels['downloads-invalid']);

      await policyInput.setInputFiles(policy);
      await metadataInput.setInputFiles(metadata.map(file => ({
        name: path.basename(file),
        mimeType: 'application/json',
        buffer: path.basename(file) === 'timestamp.json'
          ? Buffer.from(JSON.stringify({ ...JSON.parse(readFileSync(file, 'utf8')), signatures: [] }))
          : readFileSync(file),
      })));
      await button.click();
      await expect(form.getByRole('alert')).toHaveText(labels['downloads-verification-failed']);
      const publicPolicy = new URL('/updates/stable/policy-v1.json', origin).href;
      expect((await page.request.get(publicPolicy)).status()).toBe(404);

      await metadataInput.setInputFiles(metadata);
      await button.click();
      await expect(page).toHaveURL(/\/admin\/downloads\?publication=([0-9a-f-]+)#publication-\1$/);
      const release = page.locator(new URL(page.url()).hash);
      await expect(release).toContainText('1.0.0 · rel-a');
      await expect(release).toContainText(labels['downloads-verified']);
      await expect(release).toContainText(labels['downloads-active']);
      await expect(release).toContainText(staff);
      const result = await page.request.get(publicPolicy);
      expect(result.status()).toBe(200);
      expect(await result.body()).toEqual(readFileSync(policy));
      expect((await page.request.post(target, {
        data: payload, headers: { origin, 'x-csrf-token': csrf },
      })).status()).toBe(409);
      await expect(page.locator('article[id^=publication-]')).toHaveCount(1);
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);

      await page.goto(home);
      await page.getByRole('link', { name: labels['downloads-catalog'], exact: true }).click();
      const status = page.locator('#catalog-status');
      await expect(status.getByRole('heading')).toHaveText(catalog.candidates);
      await expect(status).toContainText(snapshot);
      await expect(status).toContainText(catalog.verified);
      await expect(status).toContainText(catalog.inactive);
      await expect(page.locator('main')).toContainText(catalog.unavailable);
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    } finally {
      await context.close();
    }
  });
});

test('i18n-parity for download publication and catalog status', () => {
  for (const [directory, keys] of [
    ['templates/admin', ['downloads-title', 'downloads-verification-failed', 'downloads-catalog']],
    ['templates/admin/world', ['candidates', 'activation', 'selected_for_publication', 'unavailable']],
  ]) {
    const [en, pl] = ['en', 'pl'].map(lang => messages(directory, lang));
    expect(Object.keys(en).sort()).toEqual(Object.keys(pl).sort());
    for (const key of keys) {
      expect(en[key]).toBeTruthy();
      expect(pl[key]).toBeTruthy();
    }
  }
});
