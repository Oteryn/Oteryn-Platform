// Run the real portal through the cross-repository harness with local contract settings.
import { randomBytes } from 'node:crypto';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { SMTPServer } from 'smtp-server';

const e2e = fileURLToPath(new URL('../', import.meta.url));
// An existing Playwright worker can use the isolated portal through exercise.
export async function run({ config, args, portalOptions = {}, harnessEnv = {}, browser = false }, exercise) {
  const binary = path.resolve(process.env.PORTAL_E2E_BINARY
    || path.join(e2e, '../target/debug/oteryn-portal'));
  fs.accessSync(binary, fs.constants.X_OK);
  // run.mjs trusts the shared browser harness CA before launching Playwright.
  const browserTls = browser ? {
    OTERYN_NATIVE_TLS_CERT_FILE: process.env.OTERYN_INTERNAL_TLS_CERT_FILE,
    OTERYN_NATIVE_TLS_KEY_FILE: process.env.OTERYN_INTERNAL_TLS_KEY_FILE,
  } : {};
  if (Object.values(browserTls).some(file => !file)) {
    throw new Error('Start the shared browser harness and launch this spec with run.mjs');
  }
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'portal-api-contracts-'));
  const output = path.join(scratch, 'harness');
  const launch = path.join(scratch, 'launch-portal.mjs');
  const seed = path.join(scratch, 'seed.json');
  const nativePort = Number(harnessEnv.PORTAL_API_NATIVE_PORT || process.env.PORTAL_API_NATIVE_PORT || 18544);
  const publicOrigin = `https://${browser ? 'localhost' : '127.0.0.1'}:${nativePort}`;
  const smtpPort = Number(harnessEnv.PORTAL_API_SMTP_PORT || process.env.PORTAL_API_SMTP_PORT || 18587);
  const smtpPassword = randomBytes(32).toString('hex');
  const clientId = '019a5f4a-1000-7000-8000-000000000001';
  const redirect = 'http://127.0.0.1:41000/callback';
  const webhookSecret = randomBytes(32).toString('hex');
  const options = {
    OTERYN_ACCOUNT_LOGIN: 'true',
    OTERYN_NATIVE_REMEMBERED_DEVICES_ENABLED: 'true',
    OTERYN_NATIVE_DEVICE_PROJECTION_AUTHORITY: 'oteryn-game-e2e',
    OTERYN_NATIVE_TLS_BIND: `127.0.0.1:${nativePort}`,
    OTERYN_SMTP_HOST: 'localhost',
    OTERYN_SMTP_PORT: String(smtpPort),
    OTERYN_SMTP_USERNAME: 'contract',
    OTERYN_SMTP_PASSWORD: smtpPassword,
    OTERYN_MAIL_FROM: 'portal@example.invalid',
    OTERYN_SMTP_FROM: 'portal@example.invalid',
    GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS: '300',
    PORTAL_ADMIN_CONTROL_ENABLED: 'true',
    PORTAL_ADMIN_CONTROL_AGENTS_JSON: JSON.stringify([{
      cn: 'oteryn-admin-agent/contract-agent', identity: { san: 'contract-agent.e2e.invalid' },
      allowed_worlds: ['01890f4e-7c00-7000-8000-000000000004'], allowed_nodes: [],
    }]),
    PORTAL_OAUTH_CONFIG: JSON.stringify({ enabled: true, clients: [{ client_id: clientId, redirect_uris: [redirect] }] }),
    PLATFORM_BUILD_REVISION: '0123456789abcdef0123456789abcdef01234567',
    PORTAL_COIN_TRANSFERS_ENABLED: 'true',
    PORTAL_COIN_PURCHASES_ENABLED: 'true',
    PORTAL_PAYMENTS_PROVIDER_WEBHOOK_ENABLED: 'true',
    PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET: webhookSecret,
    PORTAL_COIN_MARKET_ENABLED: 'true',
    PORTAL_NATIVE_ADMISSION_REQUESTS_PER_MINUTE: '8',
    PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED: 'true',
    ...portalOptions,
  };
  fs.writeFileSync(launch, `#!${process.execPath}
  import { spawn } from 'node:child_process';
  const child = spawn(${JSON.stringify(binary)}, process.argv.slice(2), {
    env: { ...process.env, ...${JSON.stringify(options)},
      OTERYN_NATIVE_TLS_CERT_FILE: process.env.OTERYN_INTERNAL_TLS_CERT_FILE,
      OTERYN_NATIVE_TLS_KEY_FILE: process.env.OTERYN_INTERNAL_TLS_KEY_FILE,
      ...${JSON.stringify(browserTls)},
      SSL_CERT_FILE: process.env.OTERYN_INTERNAL_TLS_CLIENT_CA_FILE,
    }, stdio: 'inherit',
  });
  for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => child.kill(signal));
  child.on('exit', code => process.exit(code ?? 1));
  `, { mode: 0o700 });

  const steps = [1, 2].map(number => ({
    name: `coinAccount${number}`,
    entity: 'account',
    args: { accountId: `019a65f0-0000-7000-8000-00000000000${number}`,
      email: `coin-${number}@example.invalid`, password: randomBytes(24).toString('hex') },
  }));
  steps.push({
    name: 'bootstrap', entity: 'bootstrap-intent', args: {
      accountId: steps[0].args.accountId, operationId: '01890f4e-7c00-7000-8000-000000000011',
      targetWorldId: '01890f4e-7c00-7000-8000-000000000004', requestedName: 'Aldric Vale',
      requestedSex: 'Male', requestedVocation: 'Knight', profileRevision: 'profile:1',
      rulesetRevision: 'ruleset:1', contentRevision: 'content:1', starterTemplateRevision: 'newhaven-starter-r1',
    },
  });
  steps.push({ name: 'nativeClient', entity: 'oauth-native-client', args: { clientId } });
  fs.writeFileSync(seed, JSON.stringify({ steps }), { mode: 0o600 });

  const harness = spawn(process.execPath, ['harness.mjs', '--seed-plan', seed], {
    cwd: e2e,
    env: { ...process.env, ...harnessEnv, PORTAL_E2E_BINARY: launch, PORTAL_E2E_OUT: output,
      OTERYN_PUBLIC_ORIGIN: publicOrigin,
      PRODUCTS_ENTITLEMENTS_STORE_SOURCE_AUTHORITY: 'oteryn-game/store-consumer' },
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  const stopped = once(harness, 'close');
  let smtp;
  let playwright;
  let playwrightStopped;
  const interrupted = new Error('API contract runner interrupted');
  const { promise: interruption, reject: interrupt } = Promise.withResolvers();
  const onInterrupt = () => { process.exitCode = 130; interrupt(interrupted); };
  const onTerminate = () => { process.exitCode = 143; interrupt(interrupted); };
  process.on('SIGINT', onInterrupt);
  process.on('SIGTERM', onTerminate);
  try {
    // The harness emits its report after migrations, readiness and CLI seeding.
    await Promise.race([
      interruption,
      once(harness.stdout, 'data'),
      stopped.then(([code]) => { throw new Error(`Portal harness exited before readiness (${code})`); }),
    ]);
    smtp = new SMTPServer({
      cert: fs.readFileSync(path.join(output, 'secrets', 'internal-server.pem')),
      key: fs.readFileSync(path.join(output, 'secrets', 'internal-server.key')),
      onAuth(auth, _session, done) {
        if (auth.username === 'contract' && auth.password === smtpPassword) done(null, { user: 'contract' });
        else done(new Error('Invalid local SMTP credentials'));
      },
      onData(stream, _session, done) {
        stream.on('end', done);
        stream.resume();
      },
    });
    await Promise.race([interruption, new Promise((resolve, reject) => {
      smtp.once('error', reject);
      smtp.listen(smtpPort, '127.0.0.1', resolve);
    })]);
    const databaseURL = JSON.parse(fs.readFileSync(path.join(output, 'secrets', 'env.json'))).OTERYN_DATABASE_URL;
    if (exercise) {
      await Promise.race([interruption, exercise({ origin: publicOrigin, databaseURL, account: steps[0].args })]);
      return;
    }
    playwright = spawn(process.execPath,
      ['node_modules/@playwright/test/cli.js', 'test', '-c', config, ...args],
      { cwd: e2e, env: { ...process.env, PORTAL_API_CONTRACT_HARNESS: output,
        PORTAL_E2E_ORIGIN: publicOrigin,
        PORTAL_API_LOGIN_PASSWORD: steps[0].args.password,
        PORTAL_API_OTHER_LOGIN_PASSWORD: steps[1].args.password,
        PORTAL_API_WEBHOOK_SECRET: webhookSecret,
        NODE_EXTRA_CA_CERTS: path.join(output, 'certs', 'ca.pem'),
        PORTAL_E2E_DATABASE_URL: databaseURL }, stdio: 'inherit' });
    playwrightStopped = once(playwright, 'close');
    const [code] = await Promise.race([interruption, playwrightStopped]);
    process.exitCode = code ?? 1;
  } catch (error) {
    if (error !== interrupted) throw error;
  } finally {
    try {
      if (playwright?.exitCode === null && playwright.signalCode === null) playwright.kill('SIGINT');
      await playwrightStopped;
    } finally {
      harness.kill('SIGTERM');
      try {
        await stopped;
        if (smtp) await new Promise(resolve => smtp.close(resolve));
      } finally {
        fs.rmSync(scratch, { recursive: true, force: true });
        process.off('SIGINT', onInterrupt);
        process.off('SIGTERM', onTerminate);
      }
    }
  }
}
