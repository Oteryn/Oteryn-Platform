import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { assertRefusalState, privateDatabase } from './database.mjs';

const id = '019a541b-a000-7000-8000-000000000001';
const offer = {
  client_build: 'local-authz', client_platform: 'windows',
  transports: [{ protocol_major: 1, transport_profile: 1, alpn: 'oteryn-game/1' }],
};
const probes = [
  ['/v1/login', { protocol_version: 2, game_login_ticket: '', attempt_ref: id, character_id: id, offer },
    'NATIVE_LOGIN_REQUEST_MALFORMED'],
  ['/v1/recover', { protocol_version: 3, recovery_handle: '', attempt_ref: id, character_id: id, offer },
    'NATIVE_LOGIN_UNSUPPORTED_VERSION'],
  ['/v1/recover/revoke', { protocol_version: 3, recovery_handle: '' }, 'NATIVE_LOGIN_UNSUPPORTED_VERSION'],
];

export async function exerciseGateway(request, token, routes) {
  const reportPath = process.env.PORTAL_E2E_HARNESS_REPORT;
  const report = JSON.parse(readFileSync(reportPath, 'utf8'));
  const origin = new URL(report.baseUrls.gateway);
  if (origin.hostname !== '127.0.0.1' || origin.protocol !== 'http:') {
    throw new Error('Authorization checks require the local gateway harness');
  }
  const db = privateDatabase(reportPath, report);
  try {
    for (const [path, data, code] of probes) {
      const route = routes.find(r => r.listener === 'gateway' && r.method === 'POST' && r.path === path);
      const { guard, refusal } = route.registrations[0];
      expect(guard.kind).toBe('client-credentials');
      await test.step(`POST ${path} on gateway`, async () => {
        const before = await db.snapshot();
        const response = await request.post(new URL(path, origin).href, {
          data, headers: { 'x-csrf-token': token }, maxRedirects: 0,
        });
        try {
          expect(response.status(), refusal.source).toBe(refusal.status);
          expect((await response.json()).error.code).toBe(code);
          assertRefusalState(before, await db.snapshot());
        } finally {
          await response.dispose();
        }
      });
    }
  } finally {
    await db.close();
  }
}
