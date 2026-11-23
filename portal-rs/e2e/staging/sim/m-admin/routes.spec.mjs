import { expect, routeStatus, test, waitUntilReady } from './helpers.mjs';

// The staging hostname currently refuses the staff surface before session and form processing.
// No request uses a real resource identifier or can create a valid administrative command.
const nonexistent = '00000000-0000-4000-8000-000000000001';
const groups = {
  'panel, roles, players and access console': [
    ['GET', '/admin'],
    ['GET POST', '/admin/roles'],
    ['GET', '/admin/players'],
    ['GET POST', '/admin/players/{id}'],
    ['GET POST', '/admin/console/players'],
    ['GET', '/admin/console/audit'],
    ['GET', '/admin/console/access'],
    ['GET', '/admin/console/approvals'],
  ],
  'game operations console and staff controls': [
    ['GET POST', '/console/dashboard'],
    ['GET POST', '/console/worlds'],
    ['GET POST', '/console/nodes'],
    ['GET POST', '/console/database'],
    ['GET POST', '/admin/control/step-up'],
    ['GET POST', '/admin/control/approvals/{id}'],
  ],
  'analytics pages and data': ['', '/creatures', '/activity', '/levels', '/health', '/economy']
    .flatMap((area) => [
      ['GET', `/admin/analytics${area}`],
      ['GET', `/admin/analytics${area}/data`],
    ]),
  'staff support, reports and enforcement': [
    ['GET', '/admin/support/tickets'],
    ['GET POST', '/admin/support/tickets/{id}'],
    ['GET', '/admin/moderation/reports'],
    ['GET POST', '/admin/moderation/reports/{id}'],
    ['GET', '/admin/moderation/enforcement'],
    ['GET', '/admin/moderation/enforcement/create'],
    ['GET POST', '/admin/moderation/enforcement/{id}'],
    ['GET', '/{lang}/admin/support/tickets'],
    ['GET', '/{lang}/admin/support/tickets/{id}'],
    ['POST', '/{lang}/admin/support/tickets/{id}/reply'],
    ['POST', '/{lang}/admin/support/tickets/{id}/status'],
  ],
  'commerce, Bazaar, world and store codes': [
    ['GET', '/admin/commerce'],
    ['POST', '/admin/commerce/{action}', { action: 'grant' }],
    ['POST', '/admin/commerce/{action}', { action: 'revoke' }],
    ['POST', '/admin/commerce/{action}', { action: 'adjust' }],
    ['POST', '/admin/commerce/{action}', { action: 'reconcile' }],
    ['POST', '/admin/commerce/{action}', { action: 'refund' }],
    ['POST', '/admin/commerce/approvals/{id}'],
    ['POST', '/admin/commerce/cancellations/{id}'],
    ['GET POST', '/admin/{lang}/bazaar'],
    ['GET POST', '/admin/world'],
    ['GET POST', '/{lang}/admin/store/codes'],
  ],
  'party board and staff moderation': [
    ['GET', '/admin/{lang}/party'],
    ['POST', '/admin/{lang}/party/{id}/moderate'],
    ['GET POST', '/{lang}/party'],
    ['POST', '/{lang}/party/{id}/report'],
    ['POST', '/{lang}/party/{id}/remove'],
  ],
  'admin content and wiki': [
    ['GET', '/admin/pages'],
    ['GET POST', '/admin/pages/{slug}/{locale}', { slug: 'terms' }],
    ['GET POST', '/admin/news'],
    ['GET', '/admin/news/create'],
    ['GET', '/admin/news/{id}/edit'],
    ['POST PUT DELETE', '/admin/news/{id}'],
    ['POST', '/admin/news/{id}/delete'],
    ['GET POST', '/admin/media'],
    ['GET', '/admin/media/{id}/content'],
    ['GET', '/admin/media/{id}/thumbnail'],
    ['DELETE', '/admin/media/{id}'],
    ['POST', '/admin/media/{id}/delete'],
    ['GET POST', '/admin/events'],
    ['GET', '/admin/events/create'],
    ['GET', '/admin/events/{id}/edit'],
    ['PUT POST', '/admin/events/{id}'],
    ['PUT POST', '/admin/events/{id}/status'],
    ['GET POST', '/admin/announcements'],
    ['GET', '/admin/announcements/create'],
    ['GET POST', '/admin/announcements/{id}/edit'],
    ['GET', '/admin/wiki'],
    ['GET', '/admin/wiki/articles/create'],
    ['GET POST', '/admin/wiki/articles'],
    ['GET', '/admin/wiki/articles/{id}/edit'],
    ['POST', '/admin/wiki/articles/{id}'],
    ['GET', '/admin/wiki/articles/{id}/revisions'],
    ['GET', '/admin/wiki/articles/{id}/preview/{locale}'],
    ['POST', '/admin/wiki/articles/{id}/revisions/{source}/restore'],
    ['POST', '/admin/wiki/articles/{id}/{action}', { action: 'submit-review' }],
    ['POST', '/admin/wiki/articles/{id}/{action}', { action: 'return-draft' }],
    ['POST', '/admin/wiki/articles/{id}/{action}', { action: 'publish' }],
    ['POST', '/admin/wiki/articles/{id}/{action}', { action: 'unpublish' }],
    ['POST', '/admin/wiki/articles/{id}/{action}', { action: 'archive' }],
  ],
};

test.beforeEach(async ({ request }) => waitUntilReady(request));

for (const [scenario, routes] of Object.entries(groups)) {
  test(`unavailable on staging: ${scenario}`, async ({ request }, testInfo) => {
    const lang = testInfo.project.metadata.language;
    for (const [methods, pattern, parameters] of routes) {
      const values = { lang, locale: lang, id: nonexistent, source: '1', ...parameters };
      const path = pattern.replaceAll(/\{(\w+)\}/g, (placeholder, key) => {
        if (!(key in values)) throw new Error(`Missing parameter for ${placeholder}`);
        return values[key];
      });
      for (const method of methods.split(' ')) {
        await test.step(`${method} ${path}`, async () => {
          expect.soft(await routeStatus(request, method, path), `${method} ${path}`).toBe(404);
        });
      }
    }
  });
}
