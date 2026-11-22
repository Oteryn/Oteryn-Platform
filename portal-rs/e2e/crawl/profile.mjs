import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { parseEnv } from 'node:util';
import { startOutboxBrowser } from '../support/outbox-browser.mjs';

export async function startCrawlBrowser({ upstreamPort, certificate, caFile, env, out }) {
  // Apply fixed settings after test.env so shell overrides cannot enable a
  // producer, point the portal outside loopback, or disable a route being tested.
  Object.assign(env, parseEnv(fs.readFileSync(new URL('./profile.env', import.meta.url), 'utf8')), {
    OTERYN_ENV: 'testing',
    APP_ENV: 'testing',
    OTERYN_WEBAUTHN_RP_ID: 'localhost',
    OTERYN_ATLAS_DIRECTORY: path.join(out, 'atlas'),
    OTERYN_RESET_DELIVERY_KEY: crypto.randomBytes(32).toString('hex'),
    OTERYN_NOTIFY_UNSUBSCRIBE_KEY: crypto.randomBytes(32).toString('hex'),
    OTERYN_SMTP_HOST: 'localhost',
    OTERYN_SMTP_USERNAME: 'portal-crawl',
    OTERYN_SMTP_PASSWORD: crypto.randomBytes(32).toString('hex'),
    OTERYN_SMTP_FROM: 'portal@example.invalid',
    OTERYN_MAIL_FROM: 'portal@example.invalid',
  });
  fs.mkdirSync(env.OTERYN_ATLAS_DIRECTORY, { mode: 0o700 });
  const close = await startOutboxBrowser({ upstreamPort, certificate, caFile, env });
  env.OTERYN_ADMIN_HOST = new URL(env.PORTAL_E2E_ORIGIN).host;
  env.OTERYN_NOTIFY_UNSUBSCRIBE_URL = `${env.PORTAL_E2E_ORIGIN}/en/unsubscribe`;
  return close;
}
