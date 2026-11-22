import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import https from 'node:https';
import { dirname, join } from 'node:path';
import { expect } from '@playwright/test';
import { sourceTime } from '../outbox-helpers.mjs';

// The inventory requires a live ownership feed. Send the same protocol as the
// shared Game producer through the real mTLS-authenticated portfolio listener.
async function publish(endpoint, fields) {
  const origin = new URL('https://' + process.env.OTERYN_INTERNAL_TLS_BIND);
  if (origin.hostname !== '127.0.0.1' || process.env.OTERYN_ENV !== 'testing') {
    throw new Error('Profile fixture requires the local browser launcher');
  }
  const caFile = process.env.OTERYN_INTERNAL_TLS_CLIENT_CA_FILE;
  const scratch = dirname(caFile);
  const payload = JSON.stringify({
    contract_version: 1,
    source_authority: process.env.OTERYN_LCFA_SOURCE_AUTHORITY,
    projection_epoch: '3', // epoch used by game-closure-producer.mjs
    ...fields,
  });
  const status = await new Promise((resolve, reject) => {
    const request = https.request(new URL('/internal/v1/game-auth/native-account-characters' + endpoint, origin), {
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
  expect(status, fields.operation + ' accepted by the portfolio publisher').toBe(200);
}

export async function publishProfileWatermark() {
  const at = sourceTime();
  await publish('/watermark', {
    operation: 'PublishProjectionWatermarkV1', complete_through: at, observed_at: at,
  });
}

export async function publishOwnedCharacter(account, world) {
  const random = randomUUID();
  const id = random.slice(0, 14) + '7' + random.slice(15);
  const name = 'Fixture ' + id.slice(-12);
  await publish('', {
    operation: 'PublishAccountCharactersV1',
    account_id: account,
    projection_revision: '1',
    source_observed_at: sourceTime(),
    characters: [{ character_id: id, world_id: world, name, availability: 'AVAILABLE' }],
  });
  await publishProfileWatermark();
  return name;
}
