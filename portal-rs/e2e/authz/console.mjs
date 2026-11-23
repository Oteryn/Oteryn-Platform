import { expect, test } from '@playwright/test';
import { TOTP } from 'otpauth';
import { query } from '../staff-rbac-helpers.mjs';

const forms = [
  ['/console/dashboard', 'ops.overview.get'],
  ['/console/worlds', 'world.list'],
  ['/console/nodes', 'node.list'],
  ['/console/database', 'db.status'],
];
const searchPath = '/admin/console/players';
const stepUp = '/admin/control/step-up';
export const consoleRoutes = [...forms.map(([path]) => ['POST', path]), ['POST', searchPath], ['POST', stepUp]];

export async function exerciseConsole(request, locale, token, definition, accountId, totpSecret, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const anonymous = definition.key === 'anonymous';
  for (const [path, operation] of forms) {
    const route = routes.find(r => r.listener === 'public' && r.method === 'POST' && r.path === path);
    const { guard } = route.registrations[0];
    const allowed = guard.all.every(permission => definition.permissions.includes(permission));
    await test.step(`POST ${path}`, async () => {
      const before = allowed ? null : await snapshot();
      const response = await request.post(path, {
        form: { _csrf: token, operation, agent: 'local-authz-no-game-agent', lang: locale },
        headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
      });
      try {
        expect(response.status(), 'crates/admin/src/console/mod.rs:765').toBe(allowed ? 503 : anonymous ? 303 : 403);
        if (anonymous) expect(response.headers().location).toBe('/login');
        if (before) {
          const outerDenial = definition.role && !definition.permissions.includes('admin.access');
          await verifyRefusal(before, outerDenial ? ['admin.access.denied'] : []);
        }
      } finally {
        await response.dispose();
      }
    });
  }
  const row = routes.find(r => r.listener === 'public' && r.method === 'POST' && r.path === searchPath);
  const { guard } = row.registrations[0];
  const allowed = guard.all.every(permission => definition.permissions.includes(permission));
  await test.step(`POST ${searchPath}`, async () => {
    const before = allowed ? null : await snapshot();
    const response = await request.post(searchPath, {
      form: { _csrf: token, q: 'Local Fixture', lang: locale },
      headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
    });
    try {
      expect(response.status(), 'crates/admin/src/console/players.rs:337').toBe(allowed || anonymous ? 303 : 403);
      if (anonymous) expect(response.headers().location).toBe('/login');
      if (allowed) {
        const location = new URL(response.headers().location, origin);
        expect(location.pathname).toBe(searchPath);
        expect(location.searchParams.get('q')).toBe('Local Fixture');
      }
      if (before) {
        const outerDenial = definition.role && !definition.permissions.includes('admin.access');
        await verifyRefusal(before, outerDenial ? ['admin.access.denied'] : []);
      }
    } finally {
      await response.dispose();
    }
  });
  const control = routes.find(r => r.listener === 'public' && r.method === 'POST' && r.path === stepUp);
  const { guard: stepGuard, source } = control.registrations[0];
  const authorized = stepGuard.all.every(permission => definition.permissions.includes(permission));
  await test.step(`POST ${stepUp}`, async () => {
    const [{ id }] = await query('SELECT uuidv7() AS id');
    const before = authorized ? null : await snapshot();
    const code = authorized ? new TOTP({ secret: totpSecret, algorithm: 'SHA1', digits: 6, period: 30 }).generate() : '000000';
    const response = await request.post(`${stepUp}?request_id=${id}`, {
      form: { _csrf: token, code }, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
    });
    try {
      expect(response.status(), source).toBe(authorized ? 200 : anonymous ? 303 : 403);
      if (anonymous) expect(response.headers().location).toBe('/login');
      if (before) await verifyRefusal(before, definition.role ? ['admin.access.denied'] : []);
      if (authorized) {
        expect((await query("SELECT 1 FROM audit_log WHERE actor=$1 AND action='admin.control.totp.verified'",
          [accountId])).length).toBe(1);
      }
    } finally {
      await response.dispose();
    }
  });
}
