import { expect, request } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';

export function fixture(group, file) {
  return fs.readFileSync(new URL(`../../../docs/contracts/fixtures/${group}/${file}`, import.meta.url));
}

export function jsonBytes(value) {
  return Buffer.from(JSON.stringify(value));
}

export function withUnknown(body) {
  return jsonBytes({ ...JSON.parse(body), unexpected: true });
}

export function harness() {
  return JSON.parse(fs.readFileSync(path.join(process.env.PORTAL_API_CONTRACT_HARNESS, 'report.json')));
}

export async function connect(base, identity) {
  const report = harness();
  const baseURL = base === 'native' ? process.env.PORTAL_E2E_ORIGIN : report.baseUrls[base];
  const client = identity && report.clients[identity];
  if (identity && !client) throw new Error(`Unknown harness identity ${identity}`);
  return request.newContext({
    baseURL,
    clientCertificates: client ? [{ origin: baseURL, certPath: client.cert, keyPath: client.key }] : [],
    maxRedirects: 0,
  });
}

export async function exchange(context, method, route, data, authorization) {
  // Playwright's network error call log can contain Authorization. Preserve
  // the endpoint and method while keeping raw credentials out of diagnostics.
  try {
    return await context.fetch(route, {
      method,
      data,
      headers: {
        ...(data === undefined ? {} : { 'content-type': 'application/json' }),
        ...(authorization ? { authorization } : {}),
      },
    });
  } catch {
    throw new Error(`${method} ${route} failed at the transport layer`);
  }
}

export async function wire(response, status, body) {
  expect(response.status()).toBe(status);
  expect(await response.body()).toEqual(Buffer.from(body));
  if (body.length !== 0) expect(response.headers()['content-type']).toBe('application/json');
}
