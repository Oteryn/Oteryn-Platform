import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

export const accountBadgeRoutes = ['', '/pl', '/en'].map(prefix => ['POST', `${prefix}/account/badges`]);

export async function exerciseBadges(request, prefix, accountId, token, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const path = `${prefix}/account/badges`;
  await test.step(`POST ${path}`, async () => {
    const before = accountId ? null : await snapshot();
    const response = await request.post(path, {
      form: { badge: 'account-age', profile: '', ranking: '' },
      headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
    });
    try {
      expect(response.status(), 'crates/characters/src/badges/http.rs:71').toBe(303);
      const destination = /^(\/pl|\/en)\/(login|account\/badges)$/.exec(response.headers().location);
      expect(destination?.[2]).toBe(accountId ? 'account/badges' : 'login');
      if (before) await verifyRefusal(before);
      else {
        const [selection] = await query('SELECT badge_keys FROM account_badge_selections WHERE account_id=$1', [accountId]);
        expect(selection.badge_keys).toEqual(['account-age']);
      }
    } finally {
      await response.dispose();
    }
  });
}
