import { mirrorHead } from './head.mjs';
import { randomUUID } from 'node:crypto';
import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

const unusedId = '019a541b-a000-7000-8000-000000000001';
export const calendarRoutes = [
  ['POST', '/admin/announcements'],
  ['GET', '/admin/announcements/{id}/edit'],
  ['POST', '/admin/announcements/{id}/edit'],
  ['POST', '/admin/events'],
  ['GET', '/admin/events/{id}/edit'],
  ['POST', '/admin/events/{id}'],
  ['PUT', '/admin/events/{id}'],
  ['POST', '/admin/events/{id}/status'],
  ['PUT', '/admin/events/{id}/status'],
  ['POST', '/admin/pages/{slug}/{locale}'],
  ['GET', '/news/{slug}'],
  ['GET', '/{lang}/news/{slug}'],
  ['GET', '/en/pages/{slug}'],
  ['GET', '/pl/pages/{slug}'],
  ['GET', '/{locale}/events/{slug}'],
];

export async function exerciseCalendar(request, token, definition, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const anonymous = definition.key === 'anonymous';
  async function checked(method, pattern, target, form) {
    const route = routes.find(r => r.listener === 'public' && r.method === method && r.path === pattern);
    const { guard, source } = route.registrations[0];
    const allowed = guard.all.every(permission => definition.permissions.includes(permission));
    return test.step(`${method} ${pattern}`, async () => {
      const before = form && !allowed ? await snapshot() : null;
      const response = await request.fetch(target, {
        method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
        ...(form && { form }),
      });
      try {
        expect(response.status(), source).toBe(allowed ? form ? 303 : 200 : anonymous ? 303 : 403);
        if (method === 'GET') await mirrorHead(request, pattern, target, response);
        if (anonymous) expect(response.headers().location).toBe('/login');
        if (before) await verifyRefusal(before, definition.role ? ['admin.access.denied'] : []);
        return allowed;
      } finally {
        await response.dispose();
      }
    });
  }

  const announcement = {
    publication_state: 'draft', severity: 'info', starts_at: new Date().toISOString(),
    title: `Local announcement ${randomUUID()}`, body: 'Authorization fixture.',
  };
  const announced = await checked('POST', '/admin/announcements', '/admin/announcements', {
    ...announcement, lock_version: '0',
  });
  const notice = announced
    ? (await query('SELECT id, lock_version FROM announcements WHERE title=$1', [announcement.title]))[0]
    : { id: unusedId, lock_version: 1 };
  const noticeTarget = `/admin/announcements/${notice.id}/edit`;
  await checked('GET', '/admin/announcements/{id}/edit', noticeTarget);
  await checked('POST', '/admin/announcements/{id}/edit', noticeTarget, {
    ...announcement, lock_version: String(notice.lock_version), severity: 'warning',
  });
  if (announced) expect((await query('SELECT severity FROM announcements WHERE id=$1', [notice.id]))[0].severity)
    .toBe('warning');

  const futureDate = days => new Date(Date.now() + days * 86_400_000).toISOString().slice(0, 16);
  const event = {
    starts_at: futureDate(1), ends_at: futureDate(2),
    en_slug: `local-event-${randomUUID()}`, en_title: 'Local event',
    en_summary: 'Authorization fixture.', en_body: 'Local event body.',
  };
  const created = await checked('POST', '/admin/events', '/admin/events', event);
  const eventId = created
    ? (await query("SELECT event_id FROM event_translations WHERE locale='en' AND slug=$1", [event.en_slug]))[0].event_id
    : unusedId;
  const eventTarget = `/admin/events/${eventId}`;
  const version = async () => created
    ? String((await query('SELECT lock_version FROM events WHERE id=$1', [eventId]))[0].lock_version)
    : '1';
  await checked('GET', '/admin/events/{id}/edit', `${eventTarget}/edit`);
  for (const method of ['POST', 'PUT']) {
    await checked(method, '/admin/events/{id}', eventTarget, { ...event, lock_version: await version() });
  }
  for (const [method, status] of [['POST', 'published'], ['PUT', 'draft']]) {
    await checked(method, '/admin/events/{id}/status', `${eventTarget}/status`, { status, lock_version: await version() });
    if (created) expect((await query('SELECT status FROM events WHERE id=$1', [eventId]))[0].status).toBe(status);
  }

  const slug = `local-authz-${randomUUID()}`;
  const pageTarget = `/admin/pages/${slug}/en`;
  const saved = await checked('POST', '/admin/pages/{slug}/{locale}', pageTarget, {
    action: 'save', title: 'Local authorization draft', body: 'Local authorization fixture.',
  });
  if (saved) expect((await query('SELECT id FROM pages WHERE slug=$1', [slug])).length).toBe(1);
}

export async function exercisePublishedContent(request, locale, routes) {
  const slug = `local-authz-${randomUUID()}`;
  const title = 'Local public authorization fixture';
  await query(`
    INSERT INTO news(slug,title,body,published_at,title_pl,body_pl,translation_published_at,translation_source_updated_at)
    VALUES($1,$2::text,$2::text,now(),$2::text,$2::text,now(),now())
  `, [slug, title]);
  await query(`
    WITH page AS (INSERT INTO pages(id,slug) VALUES(uuidv7(),$1) RETURNING id),
    source AS (
      INSERT INTO editorial_pages(page_id,locale,title,body,published_at)
      SELECT id,'en',$2::text,$2::text,now() FROM page RETURNING page_id,revision
    )
    INSERT INTO editorial_pages(page_id,locale,title,body,published_at,source_revision)
    SELECT page_id,'pl',$2::text,$2::text,now(),revision FROM source
  `, [slug, title]);
  await query(`
    WITH staff AS (SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'),
    event AS (
      INSERT INTO events(status,starts_at,ends_at,created_by,updated_by,published_by)
      SELECT 'published',now(),now()+interval '1 day',id,id,id FROM staff RETURNING id
    )
    INSERT INTO event_translations(event_id,locale,title,slug,summary,body)
    SELECT id,l.locale,$2,$1,$2,$2 FROM event CROSS JOIN (VALUES('en'),('pl')) AS l(locale)
  `, [slug, title]);
  for (const pattern of calendarRoutes.filter(([method, path]) => method === 'GET' && !path.startsWith('/admin'))) {
    const [method, path] = pattern;
    const route = routes.find(r => r.listener === 'public' && r.method === method && r.path === path);
    const { guard, source } = route.registrations[0];
    expect(guard.kind, source).toBe('anonymous');
    const target = path.replace(/\{(?:locale|lang)\}/, locale).replace('{slug}', slug);
    await test.step(`${method} ${path}`, async () => {
      const response = await request.get(target, { maxRedirects: 0 });
      try {
        expect(response.status(), source).toBe(200);
        await mirrorHead(request, path, target, response);
        expect((await response.text()).includes(title), 'published fixture rendered').toBe(true);
      } finally {
        await response.dispose();
      }
    });
  }
}
