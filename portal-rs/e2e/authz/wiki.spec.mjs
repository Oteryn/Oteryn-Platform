import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { waitForWikiBudget } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { actorDefinitions, csrfPage, openActor, sessionHash as hashSession } from './actors.mjs';
import { assertRefusalState, database } from './database.mjs';

const { routes, roles } = JSON.parse(readFileSync(new URL('./routes.json', import.meta.url), 'utf8'));
const publicRoutes = new Map(routes.filter(r => r.listener === 'public').map(r => [`${r.method} ${r.path}`, r]));
const articles = '/admin/wiki/articles';
const guide = `${articles}/{id}`;
const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';

async function guideFixture(slug, publish) {
  return (await query(`
    WITH guide AS (
      INSERT INTO wiki_guides(id,status,published_at)
      VALUES(uuidv7(), CASE WHEN $2 THEN 'published' ELSE 'draft' END, CASE WHEN $2 THEN now() END)
      RETURNING id
    )
    INSERT INTO wiki_revisions(id,guide_id,version,locale,title,slug,summary,markdown,editor,change_note)
    SELECT uuidv7(), guide.id, 1, l.locale, 'Local wiki authorization fixture', $1,
           'Local summary', 'Local article content', staff.id, 'Local fixture'
    FROM guide CROSS JOIN (VALUES('pl'),('en')) AS l(locale)
    CROSS JOIN (SELECT id FROM accounts WHERE email='bazaar-escrow@local.test') staff
    RETURNING guide_id
  `, [slug, publish]))[0].guide_id;
}

test.describe('wiki authorization', () => {
  for (const definition of actorDefinitions(roles)) {
    test(definition.key, async ({}, info) => {
      const { locale } = info.project.metadata;
      const anonymous = definition.key === 'anonymous';
      const issuer = definition.restricted
        ? (await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'"))[0].id
        : undefined;
      const { request } = await openActor(info.project.metadata.locale, definition, issuer);
      const db = database();
      try {
        const token = await csrfPage(request, anonymous ? `/${locale}/login` : `/${locale}/`);
        const sessionHash = await hashSession(request);

        async function checked(method, path, target, form, action) {
          const { guard, source } = publicRoutes.get(`${method} ${path}`).registrations[0];
          const permissions = action ? [...guard.all, guard.actions[action]] : guard.all;
          const allowed = guard.kind === 'anonymous' || permissions.every(p => definition.permissions.includes(p));
          return test.step(`${method} ${path}`, async () => {
            const before = form && !allowed ? await db.snapshot(sessionHash) : null;
            if (guard.kind === 'anonymous') await waitForWikiBudget(target);
            const response = await request.fetch(target, {
              method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
              ...(form && { form }),
            });
            try {
              expect(response.status(), source).toBe(allowed ? form ? 303 : 200 : anonymous ? 303 : 403);
              if (!allowed && anonymous) expect(response.headers().location).toBe('/login');
              if (before) assertRefusalState(before, await db.snapshot(sessionHash),
                definition.role ? ['admin.access.denied'] : []);
              if (guard.kind === 'anonymous') {
                expect((await response.text()).includes('Local wiki authorization fixture')).toBe(true);
              }
              return allowed;
            } finally {
              await response.dispose();
            }
          });
        }

        const slug = `local-wiki-${randomUUID()}`;
        const publicSlug = `${slug}-published`;
        await guideFixture(publicSlug, true);
        const draft = {};
        for (const lang of ['pl', 'en']) {
          draft[`${lang}_slug`] = slug;
          draft[`${lang}_title`] = 'Local wiki draft';
          draft[`${lang}_summary`] = 'Local authorization summary';
          draft[`${lang}_markdown`] = 'Local authorization body.';
        }
        const created = await checked('POST', articles, articles, draft);
        const id = created
          ? (await query("SELECT guide_id FROM wiki_revisions WHERE slug=$1 AND locale='en'", [slug]))[0].guide_id
          : await guideFixture(`${slug}-existing`, false);
        const target = `${articles}/${id}`;
        const version = async () => String((await query('SELECT version FROM wiki_guides WHERE id=$1', [id]))[0].version);

        await checked('GET', `${guide}/edit`, `${target}/edit`);
        await checked('GET', `${guide}/preview/{locale}`, `${target}/preview/${locale}`);
        await checked('POST', guide, target, { ...draft, lock_version: await version() });
        await checked('GET', `${guide}/revisions`, `${target}/revisions`);
        await checked('POST', `${guide}/revisions/{source}/restore`, `${target}/revisions/1/restore`, {
          lock_version: await version(),
        });
        for (const [action, permission] of [['submit-review', 'editor'], ['publish', 'publisher']]) {
          const changed = await checked('POST', `${guide}/{action}`, `${target}/${action}`, {
            lock_version: await version(),
          }, permission);
          if (changed) expect((await query('SELECT status FROM wiki_guides WHERE id=$1', [id]))[0].status)
            .toBe(action === 'publish' ? 'published' : 'in_review');
        }
        await checked('GET', '/wiki/articles/{slug}', `/wiki/articles/${publicSlug}`);
        await checked('GET', '/{locale}/wiki/articles/{slug}', `/${locale}/wiki/articles/${publicSlug}`);
      } finally {
        await db.close();
        await request.dispose();
      }
    });
  }
});
