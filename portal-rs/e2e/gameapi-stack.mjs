import { createHash, randomBytes } from 'node:crypto';
import { writeFileSync } from 'node:fs';
import path from 'node:path';
import { characterSourceAuthority } from './game-closure-producer.mjs';

export const clientId = '019a0000-0000-7000-8000-0000000c11e7';
export const origin = 'https://localhost:18443';
export const gateway = 'http://127.0.0.1:18090';
export const nativeTls = 'https://localhost:18447';
export const callback = origin + '/__oauth/callback';
export const packageId = '019a0000-0000-7000-8000-000000000123';

// The standard browser stack also supplies the real native admission and Game API listeners.
export function prepareGameApi(scratch, binary, env) {
  const token = randomBytes(32).toString('base64url');
  const tokenFile = path.join(scratch, 'gateway-token');
  const digestFile = path.join(scratch, 'gateway-token.sha256');
  writeFileSync(tokenFile, token, { mode: 0o600 });
  writeFileSync(digestFile, createHash('sha256').update(token).digest('hex'), { mode: 0o600 });
  Object.assign(env, {
    PORTAL_GAMEAPI_REAL_BINARY: binary,
    PORTAL_OAUTH_CONFIG: JSON.stringify({
      enabled: true,
      clients: [clientId, 'e2e-local'].map(client_id => ({ client_id, redirect_uris: [callback] })),
    }),
    OTERYN_NATIVE_TLS_BIND: '127.0.0.1:18447',
    OTERYN_NATIVE_TLS_CERT_FILE: path.join(scratch, 'cert.pem'),
    OTERYN_NATIVE_TLS_KEY_FILE: path.join(scratch, 'key.pem'),
    OTERYN_NATIVE_DEVICE_PROJECTION_AUTHORITY: characterSourceAuthority,
    PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET: randomBytes(32).toString('base64url'),
    PORTAL_NATIVE_ADMISSION_BIND: '127.0.0.1:18091',
    PORTAL_GATEWAY_TOKEN_SHA256_FILE: digestFile,
    PORTAL_COIN_PACKAGES: JSON.stringify([{
      id: packageId, title: 'E2E coin package', coins: 500,
      price: { minor_units: 499, currency: 'PLN', exponent: 2 }, valid_for_seconds: 300,
    }]),
    TRUSTED_PROXIES: '127.0.0.1',
  });
  return {
    ...env,
    GATEWAY_LISTEN_ADDR: new URL(gateway).host,
    GATEWAY_TRUSTED_PROXIES: '127.0.0.1',
    GATEWAY_NATIVE_LOGIN_ENABLED: 'true',
    GATEWAY_NATIVE_PROTOCOL3_ENABLED: 'false',
    GATEWAY_VERSION: 'e2e-gameapi',
    OTERYN_PLATFORM_BASE_URL: 'http://127.0.0.1:18091',
    OTERYN_PLATFORM_SERVICE_TOKEN_FILE: tokenFile,
  };
}
