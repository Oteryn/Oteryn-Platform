import { readFileSync } from 'node:fs';
import https from 'node:https';
import { dirname, join } from 'node:path';
import { expect } from '@playwright/test';
import { sourceTime } from '../outbox-helpers.mjs';

// The inventory's two profile cases require a live ownership feed. Use the
// existing launcher certificate and actual mTLS publisher. A fresh watermark
// with no snapshot for this account exercises the true missing-subject 404.
export async function publishProfileWatermark() {
  const origin = new URL('https://' + process.env.OTERYN_INTERNAL_TLS_BIND);
  if (origin.hostname !== '127.0.0.1' || process.env.OTERYN_ENV !== 'testing') {
    throw new Error('Profile fixture requires the local browser launcher');
  }
  const caFile = process.env.OTERYN_INTERNAL_TLS_CLIENT_CA_FILE;
  const scratch = dirname(caFile);
  const at = sourceTime();
  const payload = JSON.stringify({
    contract_version: 1,
    operation: 'PublishProjectionWatermarkV1',
    source_authority: process.env.OTERYN_LCFA_SOURCE_AUTHORITY,
    projection_epoch: '3', // same epoch used by game-closure-producer.mjs
    complete_through: at,
    observed_at: at,
  });
  const status = await new Promise((resolve, reject) => {
    const request = https.request(new URL('/internal/v1/game-auth/native-account-characters/watermark', origin), {
      method: 'POST',
      ca: readFileSync(caFile),
      cert: readFileSync(join(scratch, 'closure-portfolio.pem')),
      key: readFileSync(join(scratch, 'closure-portfolio-key.pem')),
      minVersion: 'TLSv1.3',
      ALPNProtocols: ['http/1.1'],
      signal: AbortSignal.timeout(5_000),
      headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(payload) },
    }, response => {
      response.once('error', reject);
      response.once('end', () => resolve(response.statusCode));
      response.resume();
    });
    request.once('error', reject);
    request.end(payload);
  });
  expect(status, 'LCFA watermark admitted using the portfolio publisher').toBe(200);
}
