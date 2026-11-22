import { execFile } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import https from 'node:https';
import { promisify } from 'node:util';
import { expect } from './fixtures.mjs';
import { login, password } from './helpers.mjs';

const execute = promisify(execFile);
export const nil = '00000000-0000-0000-0000-000000000000';
export const sourceTime = () => String(Math.floor(Date.now() / 1000));

function harnessReport() {
  const file = process.env.PORTAL_E2E_OUTBOX_REPORT;
  if (!file) throw new Error('Start the E2E harness with --outbox and run using its secrets/env.json');
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

export async function portalCommand(args) {
  const { database } = harnessReport();
  const url = new URL(process.env.OTERYN_DATABASE_URL);
  if (!['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)
    || !database.startsWith('portal_e2e_') || url.pathname !== `/${database}`) {
    throw new Error('Portal commands require the disposable harness database');
  }
  try {
    return (await execute(process.env.PORTAL_E2E_OUTBOX_BINARY, args)).stdout.trim();
  } catch {
    throw new Error(`Harness portal command failed: ${args[0]} ${args[1]}`);
  }
}

export async function seededAccount(lang) {
  const account = { email: `outbox-${crypto.randomUUID()}@example.invalid`, password: password() };
  const output = await portalCommand(['seed', 'account', JSON.stringify({ ...account, locale: lang })]);
  const { accountId } = JSON.parse(output);
  expect(accountId).toMatch(/^[\da-f-]{36}$/);
  return { ...account, id: accountId };
}

export async function seededPlayer(page, lang) {
  const account = await seededAccount(lang);
  await login(page, lang, account);
  return account;
}

export function gamePeer(identity) {
  const report = harnessReport();
  const origin = new URL(report.baseUrls.internal);
  if (origin.protocol !== 'https:' || origin.hostname !== '127.0.0.1') {
    throw new Error('Game peer requires the local harness TLS listener');
  }
  const client = report.clients[identity];
  const options = {
    method: 'POST',
    servername: report.internalServerName,
    ca: fs.readFileSync(report.caFile),
    cert: fs.readFileSync(client.cert),
    key: fs.readFileSync(client.key),
    minVersion: 'TLSv1.3',
    ALPNProtocols: ['http/1.1'],
  };
  return (route, body, { timeout = 5_000 } = {}) => {
    const raw = typeof body === 'string' ? body : JSON.stringify(body);
    return new Promise((resolve, reject) => {
      const request = https.request(new URL(route, origin), {
        ...options,
        signal: AbortSignal.timeout(timeout),
        headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(raw) },
      }, async response => {
        try {
          const chunks = [];
          for await (const chunk of response) chunks.push(chunk);
          resolve({ status: response.statusCode, headers: response.headers, raw: Buffer.concat(chunks).toString() });
        } catch (error) {
          reject(error);
        }
      });
      request.on('error', reject);
      request.end(raw);
    });
  };
}

export async function gameJson(peer, route, body) {
  const response = await peer(route, body);
  expect(response.status, route).toBe(200);
  expect(response.headers['cache-control']).toContain('no-store');
  return JSON.parse(response.raw);
}
