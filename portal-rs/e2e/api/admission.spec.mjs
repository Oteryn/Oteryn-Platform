import { expect, test } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { connect, exchange, fixture, harness, jsonBytes, wire, withUnknown } from './transport.mjs';

const vector = file => fixture('native-gateway-protocol3-v1', file);
const prefix = '/internal/v1/game-auth/';
const cases = [
  ['native-admissions', 'login-request.json'],
  ['native-recoveries', 'recover-request.json'],
  ['native-recoveries/revoke', 'recover-revoke-request.json'],
];

test('the private admission listener separates service authentication from request processing', async () => {
  const client = await connect('nativeAdmission');
  const token = fs.readFileSync(path.join(harness().outputDirectory, 'secrets', 'gateway-token'), 'utf8').trim();
  const authorization = 'Bearer ' + token;
  try {
    for (const [route, file] of cases) {
      for (const header of [undefined, 'Bearer invalid-e2e-service-token']) {
        await wire(await exchange(client, 'POST', prefix + route, vector(file), header),
          401, '{"error":"unauthorized_service"}');
      }
    }
    const route = prefix + 'native-admissions';
    await wire(await exchange(client, 'POST', route, vector('login-request.json'), authorization),
      400, vector('login-unsupported-version.json'));

    const login = { ...JSON.parse(vector('login-request.json')), protocol_version: 2 };
    const malformed = jsonBytes({
      protocol_version: 2,
      error: { code: 'NATIVE_LOGIN_REQUEST_MALFORMED', public_class: 'RETRY_LOGIN' },
      attempt_ref: login.attempt_ref,
    });
    for (let attempt = 1; attempt < 8; attempt++) {
      await wire(await exchange(client, 'POST', route, withUnknown(jsonBytes(login)), authorization),
        400, malformed);
    }
    const limited = await exchange(client, 'POST', route, withUnknown(jsonBytes(login)), authorization);
    expect(limited.status()).toBe(429);
    expect(limited.headers()['retry-after']).toBe('60');
    await wire(limited, 429, jsonBytes({
      protocol_version: 2,
      error: { code: 'NATIVE_LOGIN_RATE_LIMITED', public_class: 'TEMPORARILY_UNAVAILABLE' },
      attempt_ref: login.attempt_ref,
    }));
  } finally {
    await client.dispose();
  }
});
