import { expect } from '@playwright/test';
import { connect, exchange, fixture, wire } from './transport.mjs';

const group = 'remembered-device-v1';
const route = '/api/v1/game-auth/device-sessions';
const vector = name => fixture(group, name);
const uuidV7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

function canonicalSecret(value) {
  return /^[A-Za-z0-9_-]{43}$/.test(value) && Buffer.from(value, 'base64url').toString('base64url') === value;
}

async function deviceResponse(response, file, now, previous, characters, startedAt = now) {
  expect(response.status()).toBe(200);
  expect(response.headers()['content-type']).toBe('application/json');
  expect(response.headers()['cache-control']).toContain('no-store');
  expect(response.headers()['pragma']).toBe('no-cache');
  const bytes = await response.text();
  expect(Buffer.byteLength(bytes)).toBeLessThanOrEqual(16384);
  const actual = JSON.parse(bytes);
  const expected = JSON.parse(vector(file));
  const { device_credential: credential, family_id: family, absolute_expires_at: absolute,
    idle_expires_at: idle } = actual;
  expect(typeof credential === 'string' && credential.startsWith('otd1.')
    && canonicalSecret(credential.slice(5))).toBe(true);
  expect(family).toMatch(uuidV7);
  expect(Number.isInteger(absolute) && Number.isInteger(idle)).toBe(true);
  expect(Math.abs(idle - now - 7 * 86400)).toBeLessThan(10);
  if (previous) {
    expect(family).toBe(previous.family);
    expect(absolute).toBe(previous.absolute);
    expect(credential !== previous.credential).toBe(true);
  } else {
    expect(absolute - idle).toBe(23 * 86400);
  }
  Object.assign(expected, { device_credential: credential, family_id: family,
    absolute_expires_at: absolute, idle_expires_at: idle });
  if (file === 'ticket-response.json') {
    expect(canonicalSecret(actual.ticket)).toBe(true);
    expect(Number.isInteger(actual.expires_at)).toBe(true);
    expect(actual.expires_at).toBeGreaterThanOrEqual(startedAt + expected.expires_in);
    expect(actual.expires_at).toBeLessThanOrEqual(now + 60);
    Object.assign(expected, { ticket: actual.ticket, expires_at: actual.expires_at });
  }
  if (characters) expected.characters = characters;
  // Compare the raw wire without printing actual credentials on a mismatch.
  expect(bytes === JSON.stringify(expected)).toBe(true);
  return { credential, family, absolute };
}

export async function qualifyRememberedDevice(bearer, characters) {
  const client = await connect('native');
  const now = () => Math.floor(Date.now() / 1000);
  const operation = vector('rotate.json');
  const credential = session => 'OterynDevice ' + session.credential;
  try {
    const issued = await deviceResponse(await exchange(client, 'POST', route,
      vector('enroll.json'), bearer), 'enroll-response.json', now());
    const rotated = await deviceResponse(await exchange(client, 'POST', route + '/native-characters',
      operation, credential(issued)), 'enroll-response.json', now(), issued, characters);
    const ticketStartedAt = now();
    const ticketed = await deviceResponse(await exchange(client, 'POST', route + '/tickets',
      operation, credential(rotated)), 'ticket-response.json', now(), rotated, undefined, ticketStartedAt);
    await wire(await exchange(client, 'POST', route + '/revoke', operation, credential(ticketed)),
      200, vector('revoke-response.json'));
    await wire(await exchange(client, 'POST', route + '/tickets', operation, credential(ticketed)),
      401, vector('denied.json'));
  } finally {
    await client.dispose();
  }
}
