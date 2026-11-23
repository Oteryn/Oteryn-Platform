import { mirrorHead } from './head.mjs';
import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

export const supportRoutes = [
  ['POST', '/{lang}/support/tickets'],
  ['GET', '/{lang}/support/tickets/{id}'],
  ['POST', '/{lang}/support/tickets/{id}/reply'],
  ['POST', '/{lang}/support/tickets/{id}/read'],
  ['POST', '/{lang}/support/tickets/{id}/status'],
  ['PUT', '/{lang}/support/tickets/{id}/status'],
  ['POST', '/{lang}/support/reports'],
  ['GET', '/{lang}/support/reports/{id}'],
  ['GET', '/{lang}/admin/support/tickets/{id}'],
  ['POST', '/{lang}/admin/support/tickets/{id}/reply'],
  ['POST', '/{lang}/admin/support/tickets/{id}/status'],
  ['PUT', '/{lang}/admin/support/tickets/{id}/status'],
  ['GET', '/admin/support/tickets/{id}'],
  ['POST', '/admin/support/tickets/{id}'],
  ['GET', '/admin/moderation/reports/{id}'],
  ['POST', '/admin/moderation/reports/{id}'],
];

export async function exerciseSupport(request, locale, token, definition, routes, snapshot, verifyRefusal) {
  const anonymous = definition.key === 'anonymous';
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const uuid = async () => (await query('SELECT uuidv7() AS id'))[0].id;
  const requestKey = await uuid();
  const subject = `Local authorization ticket ${requestKey}`;
  const ticketPath = `/${locale}/support/tickets`;

  async function checked(method, path, form, allowedStatus, refusedStatus, source) {
    return test.step(`${method} ${path}`, async () => {
      const before = anonymous && method !== 'GET' ? await snapshot() : null;
      const response = await request.fetch(path, {
        method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
        ...(form && { form }),
      });
      try {
        expect(response.status(), source).toBe(anonymous ? refusedStatus : allowedStatus);
        const location = response.headers().location;
        if (method === 'GET') {
          const kind = path.startsWith(`/${locale}/support/reports/`) ? 'reports' : 'tickets';
          await mirrorHead(request, `/{lang}/support/${kind}/{id}`, path, response);
        }
        if (anonymous && (refusedStatus === 302 || refusedStatus === 303)) expect(location).toBe(`/${locale}/login`);
        if (before) await verifyRefusal(before);
        return location;
      } finally {
        await response.dispose();
      }
    });
  }

  const ticketLocation = await checked('POST', ticketPath, {
    request_key: requestKey, category: 'technical', subject, body: 'Local authorization fixture.',
  }, 302, 403, 'crates/support/src/tickets/web.rs:570');
  const ticket = anonymous ? requestKey : ticketLocation?.split('/').at(-1);
  const detail = `${ticketPath}/${ticket}`;
  if (!anonymous) expect(ticketLocation).toMatch(new RegExp(`^${ticketPath}/[a-f0-9-]+$`));
  const revision = async () => anonymous ? '1'
    : String((await query('SELECT lock_version FROM tickets WHERE id=$1', [ticket]))[0].lock_version);

  await checked('GET', detail, null, 200, 303, 'crates/web/src/chrome.rs:37');
  await checked('POST', `${detail}/reply`, {
    lock_version: await revision(), body: 'Local authorization reply.',
  }, 302, 403, 'crates/support/src/tickets/web.rs:650');

  const reply = anonymous ? requestKey
    : (await query('SELECT id FROM ticket_messages WHERE ticket_id=$1 ORDER BY id DESC LIMIT 1', [ticket]))[0].id;
  await checked('POST', `${detail}/read`, { reply }, 302, 403, 'crates/support/src/tickets/web.rs:465');
  for (const [method, status] of [['POST', 'closed'], ['PUT', 'open']]) {
    await checked(method, `${detail}/status`, { status, lock_version: await revision() },
      302, 403, 'crates/support/src/tickets/web.rs:664');
    if (!anonymous) expect((await query('SELECT status FROM tickets WHERE id=$1', [ticket]))[0].status).toBe(status);
  }

  const reportPath = `/${locale}/support/reports`;
  const target = (await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'"))[0].id;
  const reportLocation = await checked('POST', reportPath, {
    request_key: await uuid(), report_type: 'player', category: 'harassment', target_reference: target,
    evidence_summary: 'Local authorization report fixture.',
  }, 302, 302, 'crates/support/src/reports/web.rs:168');
  if (!anonymous) expect(reportLocation).toMatch(new RegExp(`^${reportPath}/[a-f0-9-]+$`));
  await checked('GET', anonymous ? `${reportPath}/${requestKey}` : reportLocation,
    null, 200, 303, 'crates/web/src/chrome.rs:38');

  const staffPath = `/${locale}/admin/support/tickets/${ticket}`;
  const cases = [
    ['GET', '', null],
    ['POST', '/reply', { body: 'Local staff authorization reply.' }],
    ['POST', '/status', { status: 'closed' }],
    ['PUT', '/status', { status: 'open' }],
  ];
  for (const [method, suffix, fields] of cases) {
    const pattern = `/{lang}/admin/support/tickets/{id}${suffix}`;
    const route = routes.find(r => r.listener === 'public' && r.method === method && r.path === pattern);
    const { guard, source } = route.registrations[0];
    const allowed = guard.all.every(permission => definition.permissions.includes(permission));
    await test.step(`${method} ${pattern}`, async () => {
      const before = fields && !allowed ? await snapshot() : null;
      const response = await request.fetch(`${staffPath}${suffix}`, {
        method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
        ...(fields && { form: { ...fields, lock_version: await revision() } }),
      });
      try {
        expect(response.status(), source).toBe(allowed ? fields ? 302 : 200 : anonymous ? 303 : 403);
        if (method === 'GET') await mirrorHead(request, pattern, `${staffPath}${suffix}`, response);
        if (anonymous) expect(response.headers().location).toMatch(/^\/(?:pl\/|en\/)?login$/);
        if (before) await verifyRefusal(before, definition.role ? ['admin.access.denied'] : []);
        if (allowed && fields) expect(response.headers().location).toBe(staffPath);
        if (allowed && fields?.status) {
          expect((await query('SELECT status FROM tickets WHERE id=$1', [ticket]))[0].status).toBe(fields.status);
        }
      } finally {
        await response.dispose();
      }
    });
  }

  const report = anonymous ? requestKey : reportLocation.split('/').at(-1);
  const staffForms = [
    {
      base: '/admin/support/tickets', id: ticket,
      form: { action: 'reply', body: 'Local staff reply through the moderation panel.' },
      lockVersion: revision,
    },
    {
      base: '/admin/moderation/reports', id: report,
      form: { action: 'moderate', status: 'reviewing', public_outcome: '', moderator_notes: 'Local authorization fixture.' },
      lockVersion: async () => anonymous ? '1'
        : String((await query('SELECT lock_version FROM player_reports WHERE id=$1', [report]))[0].lock_version),
    },
  ];
  for (const { base, id, form, lockVersion } of staffForms) {
    const url = `${base}/${id}`;
    const pattern = `${base}/{id}`;
    for (const method of ['GET', 'POST']) {
      const { guard, source } = routes.find(r => r.listener === 'public' && r.method === method && r.path === pattern)
        .registrations[0];
      const allowed = guard.all.every(permission => definition.permissions.includes(permission));
      await test.step(`${method} ${pattern}`, async () => {
        const mutation = method === 'POST';
        const version = mutation ? await lockVersion() : null;
        const before = mutation && !allowed ? await snapshot() : null;
        const response = await request.fetch(url, {
          method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
          ...(mutation && { form: { ...form, lock_version: version } }),
        });
        try {
          expect(response.status(), source).toBe(allowed ? mutation ? 303 : 200 : anonymous ? 303 : 403);
          if (!mutation) await mirrorHead(request, pattern, url, response);
          if (anonymous) expect(response.headers().location).toBe('/login');
          if (before) await verifyRefusal(before, definition.role ? ['admin.access.denied'] : []);
          if (allowed && mutation) {
            expect(response.headers().location).toBe(url);
            expect(Number(await lockVersion())).toBe(Number(version) + 1);
          }
        } finally {
          await response.dispose();
        }
      });
    }
  }
}
