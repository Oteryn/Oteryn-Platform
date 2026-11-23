import { mirrorHead } from './head.mjs';
import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

const fixture = '019a541b-a000-7000-8000-000000000001';
const cases = [
  { method: 'GET', path: '/{locale}/account/journal/reports/notes', anonymous: 303, authenticated: 404,
    source: 'crates/playerdata/src/reports.rs:140', note: true },
  { method: 'POST', path: '/{locale}/account/journal/reports/notes', anonymous: 403, authenticated: 404,
    source: 'crates/playerdata/src/reports.rs:279', note: true, form: { body: 'Local authorization note' } },
  { method: 'POST', path: '/{locale}/account/journal/reports/email', anonymous: 403, authenticated: 204,
    source: 'crates/playerdata/src/reports/http.rs:181', form: { enabled: 'true' } },
  { method: 'GET', path: '/{lang}/panel/items/{world}/{character}', anonymous: 303, authenticated: 503,
    source: 'crates/playerdata/src/items/web.rs:142' },
  { method: 'GET', path: '/{lang}/panel/history/{world}/{family}', anonymous: 303, authenticated: 503,
    source: 'crates/playerdata/src/items/web.rs:142', family: 'bank' },
  { method: 'GET', path: '/{lang}/panel/history/{world}/{family}/{character}', anonymous: 303, authenticated: 503,
    source: 'crates/playerdata/src/items/web.rs:142', family: 'market' },
  { method: 'GET', path: '/{lang}/account/characters/{character}/services', anonymous: 303, authenticated: 503,
    restricted: 403, source: 'crates/characters/src/services/browser.rs:116' },
  { method: 'POST', path: '/{lang}/account/characters/{character}/services', anonymous: 403, authenticated: 403,
    source: 'crates/characters/src/services/browser.rs:177',
    form: { request_id: fixture, action: 'schedule', confirm: 'yes' } },
];
export const journalRoutes = cases.map(({ method, path }) => [method, path]);

export async function exerciseJournal(request, locale, token, accountId, restricted, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  for (const row of cases) {
    const { method, path, form } = row;
    const { guard } = routes.find(r => r.listener === 'public' && r.method === method && r.path === path)
      .registrations[0];
    expect(guard.kind, path).toBe('logged-in');
    const status = accountId ? (restricted && row.restricted) || row.authenticated : row.anonymous;
    const target = path.replace(/\{(?:lang|locale)\}/, locale).replace('{world}', fixture)
      .replace('{character}', fixture).replace('{family}', row.family) + (row.note ? `?kind=character&id=${fixture}` : '');
    await test.step(`${method} ${path}`, async () => {
      const before = form && status !== 204 ? await snapshot() : null;
      const response = await request.fetch(target, {
        method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
        ...(form && { form: { ...form, _csrf: token } }),
      });
      try {
        expect(response.status(), row.source).toBe(status);
        if (method === 'GET') await mirrorHead(request, path, target, response);
        if (status === 303) expect(response.headers().location).toBe(`/${locale}/login`);
        if (before) await verifyRefusal(before);
        if (status === 204) {
          const [preferences] = await query('SELECT email_enabled FROM pd_report_preferences WHERE account_id=$1', [accountId]);
          expect(preferences.email_enabled).toBe(true);
        }
      } finally {
        await response.dispose();
      }
    });
  }
}
