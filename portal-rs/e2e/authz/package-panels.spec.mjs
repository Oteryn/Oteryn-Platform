import { expect, test } from '@playwright/test';
import { csrfPage, openActor } from './actors.mjs';

const staffReads = [
  ['GET', '/admin/commerce/reconcile'],
  ['GET', '/admin/operations/flags'],
  ['GET', '/admin/operations/health'],
  ['GET', '/admin/operations/rate-limits'],
  ['GET', '/admin/operations/rate-limits.csv'],
  ['GET', '/admin/operations/statistics'],
  ['GET', '/admin/payment-review'],
  ['GET', '/admin/store-activation'],
  ['GET', '/admin/store-catalog'],
  ['GET', '/console'],
  ['GET', '/en/admin/roles/confirm'],
  ['GET', '/pl/admin/roles/confirm'],
];
const missingId = '019a541b-a000-7000-8000-000000000001';
const ownerReads = [
  ['GET', '/{lang}/account/bazaar/activity', 303, 200],
  ['GET', '/{lang}/account/passkeys/manage', 303, 200],
  ['GET', '/{lang}/account/store-orders', 303, 200],
  ['GET', '/{lang}/account/store-orders/{id}', 303, 404],
  ['GET', '/{lang}/support/appeals', 403, 200],
];
const review = { id: missingId, version: '0', reason: 'evidence_reviewed' };
const staffWrites = [
  ['POST', '/admin/commerce/reconcile', review],
  ['POST', '/admin/payment-review', review],
  ['POST', '/admin/store-activation', { revision: 'missing', generation: '0', reason: 'game_catalog_verified' }],
  ['POST', '/admin/store-catalog', { current: 'missing', offer: 'missing', price: '31', reason: 'price_correction' }],
  ['POST', '/en/admin/roles/confirm'],
  ['POST', '/pl/admin/roles/confirm'],
];
const ownerWrites = [
  ['POST', '/en/account/privacy/export', 403, {}],
  ['POST', '/pl/account/privacy/export', 403, {}],
  ['POST', '/{lang}/support/appeals/{id}', 404, { message: 'Review this decision' }],
];

const targetFor = (path, locale) => path.replace('{lang}', locale).replace('{id}', missingId);

test('package panel and appeal routes enforce their browser authorization', async ({}, info) => {
  const { locale } = info.project.metadata;
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const guest = await openActor(locale, { key: 'anonymous' });
  const player = await openActor(locale, { key: 'player' });
  const read = async (request, method, path, status) => {
    const target = targetFor(path, locale);
    const response = await request.fetch(target, { method, maxRedirects: 0 });
    try {
      expect(response.status(), `${method} ${path}`).toBe(status);
      if (method === 'HEAD') expect((await response.body()).length).toBe(0);
      if (status === 303) expect(response.headers().location).toMatch(/^\/(?:pl\/|en\/)?login$/);
    } finally {
      await response.dispose();
    }
  };
  const write = async (path, fields, csrf, status) => {
    const response = await player.request.post(targetFor(path, locale), {
      form: { ...fields, ...(csrf && { _csrf: csrf }) }, headers: { origin }, maxRedirects: 0,
    });
    try {
      expect(response.status(), `POST ${path}`).toBe(status);
      expect(response.headers()['content-disposition']).toBeUndefined();
    } finally {
      await response.dispose();
    }
  };
  try {
    const csrf = await csrfPage(player.request, `/${locale}/`);
    for (const [method, path] of staffReads) {
      await read(guest.request, method, path, 303);
      await read(guest.request, 'HEAD', path, 303);
      await read(player.request, method, path, 403);
      await read(player.request, 'HEAD', path, 403);
    }
    for (const [method, path, guestStatus, playerStatus] of ownerReads) {
      await read(guest.request, method, path, guestStatus);
      await read(guest.request, 'HEAD', path, guestStatus);
      await read(player.request, method, path, playerStatus);
      await read(player.request, 'HEAD', path, playerStatus);
    }
    for (const [, path, form = { current_password: player.account.password }] of staffWrites) {
      await write(path, form, csrf, 403);
    }
    for (const [, path, status, form] of ownerWrites) {
      await write(path, form, undefined, 403);
      await write(path, form, csrf, status);
    }
  } finally {
    await Promise.all([guest.request.dispose(), player.request.dispose()]);
  }
});
