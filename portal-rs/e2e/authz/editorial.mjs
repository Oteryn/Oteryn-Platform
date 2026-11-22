import { randomUUID } from 'node:crypto';
import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';
import { csrfPage, sessionHash as hashSession } from './actors.mjs';
import { assertRefusalState, database } from './database.mjs';

const emptyId = '019a541b-a000-7000-8000-000000000001';
const fixturePng = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAHklEQVR4nGNQzrP5T03MMGrgqIGjBo4aOGrgSDUQANpkzt/P0xIDAAAAAElFTkSuQmCC', 'base64');

export const editorialRoutes = [
  ['POST', '/admin/news'],
  ['GET', '/admin/news/{id}/edit'],
  ['POST', '/admin/news/{id}'],
  ['PUT', '/admin/news/{id}'],
  ['DELETE', '/admin/news/{id}'],
  ['POST', '/admin/news/{id}/delete'],
  ['POST', '/admin/media'],
  ['GET', '/admin/media/{id}/content'],
  ['GET', '/admin/media/{id}/thumbnail'],
  ['DELETE', '/admin/media/{id}'],
  ['POST', '/admin/media/{id}/delete'],
];

function hasPermission(route, definition) {
  return route.registrations[0].guard.all.every(p => definition.permissions.includes(p));
}

export async function exerciseEditorial(request, locale, definition, publicRoutes) {
  const db = database();
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const anonymous = definition.key === 'anonymous';
  try {
    const token = await csrfPage(request, anonymous ? `/${locale}/login` : `/${locale}/`);
    const sessionHash = await hashSession(request);

    async function staffRequest(method, pattern, target, status, payload = {}) {
      const route = publicRoutes.get(`${method} ${pattern}`);
      if (!route) throw new Error(`Missing registration for ${method} ${pattern}`);
      const allowed = hasPermission(route, definition);
      await test.step(`${method} ${pattern}`, async () => {
        const before = method !== 'GET' && !allowed ? await db.snapshot(sessionHash) : null;
        const response = await request.fetch(target, {
          method, maxRedirects: 0,
          headers: { origin, 'x-csrf-token': token },
          ...payload,
        });
        try {
          expect(response.status(), `${method} ${pattern} (${route.registrations[0].source})`)
            .toBe(allowed ? status : anonymous ? 303 : 403);
          if (anonymous) expect(response.headers().location).toBe('/login');
          if (before) assertRefusalState(before, await db.snapshot(sessionHash),
            definition.role ? ['admin.access.denied'] : []);
        } finally {
          await response.dispose();
        }
      });
      return allowed;
    }

    const publication = new Date(Date.now() - 60_000).toISOString();
    const news = {
      slug: `matrix-news-${randomUUID()}`, title: 'Local news fixture', title_pl: 'Lokalna wiadomość',
      body: 'Local authorization matrix publication.', body_pl: 'Lokalna publikacja testowa.',
      published_at: publication, translation_published_at: publication,
    };
    const newsAllowed = await staffRequest('POST', '/admin/news', '/admin/news', 302, { form: news });
    const newsRow = async slug => (await query('SELECT id,updated_at::text AS revision FROM news WHERE slug=$1', [slug]))[0];
    const id = newsAllowed ? (await newsRow(news.slug)).id : emptyId;
    await staffRequest('GET', '/admin/news/{id}/edit', `/admin/news/${id}/edit`, 200);
    for (const method of ['POST', 'PUT']) {
      const row = newsAllowed ? await newsRow(news.slug) : null;
      await staffRequest(method, '/admin/news/{id}', `/admin/news/${id}`, 302, {
        form: { ...news, expected_revision: row?.revision || publication },
      });
    }
    const existing = newsAllowed ? await newsRow(news.slug) : null;
    await staffRequest('DELETE', '/admin/news/{id}', `/admin/news/${id}`, 302, {
      form: { expected_revision: existing?.revision || publication },
    });
    if (newsAllowed) {
      expect(Boolean(await newsRow(news.slug)), 'DELETE removes its news fixture').toBe(false);
      await staffRequest('POST', '/admin/news', '/admin/news', 302, { form: news });
    }
    const second = newsAllowed ? await newsRow(news.slug) : { id: emptyId, revision: publication };
    await staffRequest('POST', '/admin/news/{id}/delete', `/admin/news/${second.id}/delete`, 302, {
      form: { expected_revision: second.revision },
    });

    const alt = `Local matrix image ${randomUUID()}`;
    const upload = () => ({
      multipart: { _csrf: token, alt_text: alt, image: { name: 'local.png', mimeType: 'image/png', buffer: fixturePng } },
    });
    const mediaAllowed = await staffRequest('POST', '/admin/media', '/admin/media', 302, upload());
    const mediaId = async () => (await query('SELECT id FROM media WHERE alt_text=$1', [alt]))[0]?.id;
    const image = mediaAllowed ? await mediaId() : emptyId;
    for (const variant of ['content', 'thumbnail']) {
      await staffRequest('GET', `/admin/media/{id}/${variant}`, `/admin/media/${image}/${variant}`, 200);
    }
    await staffRequest('DELETE', '/admin/media/{id}', `/admin/media/${image}`, 302);
    if (mediaAllowed) {
      expect(Boolean(await mediaId()), 'DELETE removes its image fixture').toBe(false);
      await staffRequest('POST', '/admin/media', '/admin/media', 302, upload());
    }
    const secondImage = mediaAllowed ? await mediaId() : emptyId;
    await staffRequest('POST', '/admin/media/{id}/delete', `/admin/media/${secondImage}/delete`, 302);
  } finally {
    await db.close();
  }
}
