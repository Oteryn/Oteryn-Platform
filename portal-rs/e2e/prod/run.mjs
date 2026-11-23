import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

async function run() {
  // Node enables its own debug logger before loading this launcher.
  if (process.env.NODE_DEBUG) {
    console.error('Unset NODE_DEBUG before running the production monitor.');
    return 1;
  }
  const args = process.argv.slice(2);
  const seededPrivacy = args[0] === '--seeded-privacy';
  const seededAuth = args[0] === '--seeded-auth';
  const productionAuth = args[0] === '--production-auth';
  if (seededPrivacy || seededAuth || productionAuth) args.shift();
  if (args.length > 1 || args.some((arg) => !/^--project=(pl-1440|en-390)$/.test(arg))) {
    console.error('Usage: node prod/run.mjs [--production-auth|--seeded-auth|--seeded-privacy] [--project=pl-1440|--project=en-390]');
    return 1;
  }

  const { OTERYN_MONITOR_EMAIL: email, OTERYN_MONITOR_PASSWORD: password,
    OTERYN_MONITOR_TOTP_SEED: secret, MONITOR_BASE_URL: origin } = process.env;
  if (productionAuth) {
    if (origin && new URL(origin).href !== 'https://oteryn.com/') {
      console.error('Production sign-in requires the canonical portal origin.');
      return 1;
    }
    if (!email || !password || !secret) {
      console.log('skipped: monitor account not configured');
      return 0;
    }
  }

  const dir = await mkdtemp(join(tmpdir(), 'portal-prod-monitor-'));
  try {
    const env = Object.fromEntries(Object.entries(process.env).filter(([key]) =>
      key === 'PLAYWRIGHT_BROWSERS_PATH'
      || !/^(DEBUG|NODE_DEBUG|NODE_OPTIONS$|PW|PLAYWRIGHT_|OTERYN_MONITOR_)/i.test(key),
    ));
    env.PLAYWRIGHT_NO_COPY_PROMPT = '1';
    env.MONITOR_RUN_DIRECTORY = dir;
    env.MONITOR_LOCAL_AUTH = seededAuth ? '1' : '0';
    env.MONITOR_PRODUCTION_AUTH = productionAuth ? '1' : '0';
    if (productionAuth) {
      env.MONITOR_BASE_URL = 'https://oteryn.com';
      env.MONITOR_EMAIL = email;
      env.MONITOR_PASSWORD = password;
      env.MONITOR_TOTP_SECRET = secret;
    }
    const counter = join(dir, 'requests');
    await writeFile(counter, '0');
    const cli = fileURLToPath(new URL('../node_modules/@playwright/test/cli.js', import.meta.url));
    const configName = seededPrivacy ? 'playwright.seeded-privacy.config.mjs' : 'playwright.prod.config.mjs';
    const config = fileURLToPath(new URL(configName, import.meta.url));
    const child = spawn(process.execPath, [cli, 'test', '--config', config, ...args], {
      env, stdio: 'ignore',
    });
    const [code] = await once(child, 'close');
    const diagnostics = await readFile(join(dir, 'diagnostics'), 'utf8').catch(() => '');
    for (const line of diagnostics.split('\n')) {
      if (/^(pl-1440|en-390) (passed|failed at [A-Za-z0-9 _-]+)$/.test(line)
          || /^Diagnostic: [A-Za-z0-9 /_:(),.-]{1,160}$/.test(line)) console.log(line);
    }
    const count = await readFile(counter, 'utf8');
    if (!seededPrivacy && /^\d{1,2}$/.test(count)) console.log(`Monitor sent ${count} requests.`);
    if (code !== 0) console.error(seededPrivacy ? 'Seeded Privacy checks failed.' : 'Production monitor failed.');
    return code === 0 ? 0 : 1;
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
}

try {
  process.exitCode = await run();
} catch {
  console.error('Production monitor launcher failed.');
  process.exitCode = 1;
}
