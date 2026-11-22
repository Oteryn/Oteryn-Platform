import { spawn } from 'node:child_process';
import { generateKeyPairSync, randomBytes } from 'node:crypto';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import http from 'node:http';
import https from 'node:https';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { SMTPServer } from 'smtp-server';

const here = path.dirname(fileURLToPath(import.meta.url));
const e2e = path.dirname(here);
const binary = path.resolve(process.env.PORTAL_E2E_BINARY || path.join(e2e, '../target/debug/oteryn-portal'));
const scratch = await mkdtemp(path.join(tmpdir(), 'portal-abuse-'));
const out = path.join(scratch, 'harness');
const port = Number(process.env.PORTAL_E2E_ABUSE_HTTPS_PORT || '18443');
const origin = `https://localhost:${port}`;
const settingsFile = path.join(scratch, 'settings.json');
const fenceFile = path.join(scratch, 'bazaar-fence.json');
const keyFile = path.join(scratch, 'bazaar-keys.json');
const wrapper = path.join(scratch, 'portal');
const world = '019a0000-0000-7000-8000-00000000e001';
let harness, proxy, smtp, browser;
for (const signal of ['SIGINT', 'SIGTERM']) {
  process.once(signal, () => {
    browser?.kill(signal);
    harness?.kill('SIGTERM');
  });
}

function status(child) {
  return new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', (code, signal) => resolve(code ?? (signal ? 1 : 0)));
  });
}

try {
  const smtpPassword = randomBytes(24).toString('hex');
  const key = generateKeyPairSync('ed25519').publicKey.export({ format: 'jwk' });
  await writeFile(fenceFile, JSON.stringify({
    projection_epoch: 1,
    recovery_generation: 1,
    incarnation: '019a0000-0000-7000-8000-00000000e002',
  }));
  await writeFile(keyFile, JSON.stringify([{
    kid: 'abuse-browser', public_key_hex: Buffer.from(key.x, 'base64url').toString('hex'),
  }]));
  await writeFile(settingsFile, JSON.stringify({
    OTERYN_ACCOUNT_LOGIN: 'true',
    OTERYN_SMTP_HOST: 'localhost',
    OTERYN_SMTP_USERNAME: 'local',
    OTERYN_SMTP_PASSWORD: smtpPassword,
    OTERYN_MAIL_FROM: 'portal@local.test',
    OTERYN_SMTP_FROM: 'portal@local.test',
    SSL_CERT_FILE: path.join(out, 'certs/ca.pem'),
    OTERYN_SUPPORT_TICKETS: 'true',
    OTERYN_REPORTS_ENABLED: 'true',
    PORTAL_PARTY_ENABLED: 'true',
    PORTAL_REFERRALS_ENABLED: 'true',
    OTERYN_ACCOUNT_EXPORT_ENABLED: 'true',
    OTERYN_NOTIFY_UNSUBSCRIBE_KEY: randomBytes(32).toString('hex'),
    OTERYN_NOTIFY_UNSUBSCRIBE_URL: `${origin}/pl/unsubscribe`,
    OTERYN_BAZAAR_ENABLED: 'true',
    OTERYN_BAZAAR_INTERNAL_BASE_URL: origin,
    OTERYN_BAZAAR_CLIENT_SPKI_SHA256: randomBytes(32).toString('hex'),
    OTERYN_BAZAAR_RECEIPT_CLIENT_SPKI_SHA256: randomBytes(32).toString('hex'),
    OTERYN_BAZAAR_RECEIPT_AUTHORITY: 'abuse-browser-character',
    OTERYN_BAZAAR_SOURCE_AUTHORITY: 'abuse-browser-bazaar',
    OTERYN_BAZAAR_FENCE_FILE: fenceFile,
    OTERYN_BAZAAR_SNAPSHOT_KEYS_FILE: keyFile,
    OTERYN_BAZAAR_WORLD_IDS: world,
  }), { mode: 0o600 });
  // The shared harness forwards only the names in test.env. Overlay the feature
  // settings on its real binary for this isolated suite.
  await writeFile(wrapper, `#!${process.execPath}
const { readFileSync } = require('node:fs');
const settings = JSON.parse(readFileSync(${JSON.stringify(settingsFile)}));
process.execve(${JSON.stringify(binary)}, [${JSON.stringify(binary)}, ...process.argv.slice(2)], { ...process.env, ...settings });
`, { mode: 0o700 });
  harness = spawn(process.execPath, ['harness.mjs'], {
    cwd: e2e,
    env: { ...process.env, OTERYN_PUBLIC_ORIGIN: origin, PORTAL_E2E_OUT: out, PORTAL_E2E_BINARY: wrapper },
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  const harnessExit = status(harness);
  const ready = new Promise((resolve, reject) => {
    let output = '';
    harness.stdout.setEncoding('utf8').on('data', chunk => {
      output += chunk;
      if (output.includes('\n')) {
        try { resolve(JSON.parse(output)); } catch { reject(new Error('Invalid harness readiness report')); }
      }
    });
  });
  const report = await Promise.race([ready, harnessExit.then(code => {
    throw new Error(`Harness exited before readiness (${code})`);
  })]);
  const environment = JSON.parse(await readFile(path.join(out, 'secrets/env.json')));
  const tls = {
    cert: await readFile(environment.OTERYN_INTERNAL_TLS_CERT_FILE),
    key: await readFile(environment.OTERYN_INTERNAL_TLS_KEY_FILE),
  };
  smtp = new SMTPServer({
    ...tls,
    onAuth(auth, _session, done) {
      if (auth.username === 'local' && auth.password === smtpPassword) done(null, { user: 'local' });
      else done(new Error('Invalid local SMTP credentials'));
    },
    onData(stream, _session, done) {
      stream.once('error', done);
      stream.once('end', () => done());
      stream.resume();
    },
  });
  await new Promise((resolve, reject) => {
    smtp.once('error', reject);
    smtp.listen(587, '127.0.0.1', resolve);
  });
  const publicPort = new URL(report.baseUrls.public).port;
  proxy = https.createServer(tls, (request, response) => {
    const upstream = http.request({
      host: '127.0.0.1', port: publicPort, path: request.url,
      method: request.method, headers: request.headers,
    }, result => {
      response.writeHead(result.statusCode, result.headers);
      result.pipe(response);
    });
    upstream.on('error', () => response.destroy());
    request.pipe(upstream);
  });
  await new Promise((resolve, reject) => {
    proxy.once('error', reject);
    proxy.listen(port, '127.0.0.1', resolve);
  });
  browser = spawn(process.execPath, [
    'node_modules/@playwright/test/cli.js', 'test', '--config=abuse/playwright.config.mjs',
    ...process.argv.slice(2),
  ], {
    cwd: e2e,
    env: {
      ...process.env,
      PORTAL_E2E_ORIGIN: origin,
      PORTAL_E2E_DATABASE_URL: environment.OTERYN_DATABASE_URL,
      PORTAL_E2E_BINARY: binary,
    },
    stdio: 'inherit',
  });
  process.exitCode = await Promise.race([status(browser), harnessExit.then(code => {
    browser.kill('SIGTERM');
    throw new Error(`Harness stopped during browser qualification (${code})`);
  })]);
} finally {
  smtp?.close();
  proxy?.closeAllConnections();
  proxy?.close();
  if (harness && harness.exitCode === null && harness.signalCode === null) {
    const closed = status(harness);
    harness.kill('SIGTERM');
    await closed;
  }
  if (process.env.PORTAL_E2E_ABUSE_KEEP_LOGS === '1') {
    console.error(`Private abuse harness logs retained in ${scratch}`);
  } else {
    await rm(scratch, { recursive: true, force: true });
  }
}
