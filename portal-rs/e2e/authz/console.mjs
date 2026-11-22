import { expect, test } from '@playwright/test';

const forms = [
  ['/console/dashboard', 'ops.overview.get'],
  ['/console/worlds', 'world.list'],
  ['/console/nodes', 'node.list'],
  ['/console/database', 'db.status'],
];
export const consoleRoutes = forms.map(([path]) => ['POST', path]);

export async function exerciseConsole(request, locale, token, definition, routes, snapshot, verifyRefusal) {
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
}
