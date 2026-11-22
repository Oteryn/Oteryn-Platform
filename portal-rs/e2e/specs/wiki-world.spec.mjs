import { expectUnpublishedCatalog } from '../catalog-setup.mjs';
import { test, expect } from '../fixtures.mjs';
import { waitForWikiBudget } from '../helpers.mjs';

const notices = {
  pl: {
    missing: 'Nie znaleziono strony.',
    unavailable: 'Dane świata są obecnie niedostępne.',
    invalid: 'Nieprawidłowe filtry lub zapytanie. Użyj 2–80 znaków i strony od 1 do 1000.',
  },
  en: {
    missing: 'Page not found.',
    unavailable: 'World data is currently unavailable.',
    invalid: 'Invalid filters or search query. Use 2–80 characters and a page from 1 to 1000.',
  },
};

async function expectWikiNotice(page, path, status, heading) {
  await waitForWikiBudget(path);
  const response = await page.goto(path);
  expect(response.request().redirectedFrom(), `direct navigation to ${path}`).toBeNull();
  expect(response.status(), path).toBe(status);
  await expect(page).toHaveURL(url => url.pathname + url.search === path);
  const cacheControl = response.headers()['cache-control'] ?? '';
  expect(cacheControl.toLowerCase().split(',').map(directive => directive.trim())).toContain('no-store');
  await expect(page.locator('html')).toHaveAttribute('lang', path.split('/')[1]);
  await expect(page.locator('header#portal-header.om-interior')).toBeVisible();
  await expect(page.locator('footer.foot-interior')).toBeVisible();
  await expect(page.getByRole('main').getByRole('heading', { level: 1, name: heading, exact: true })).toBeVisible();
}

test('wiki lists and search show the designed unavailable view before admission', async ({ page }, info) => {
  await expectUnpublishedCatalog();
  const lang = info.project.metadata.locale;
  for (const suffix of ['creatures', 'items', 'spells', 'search?q=rat']) {
    await expectWikiNotice(page, `/${lang}/wiki/${suffix}`, 503, notices[lang].unavailable);
  }
});

test('wiki rejects invalid search and list input before consulting the catalog', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  for (const suffix of ['search?q=x', 'items?page=0', 'spells?page=not-a-number', 'creatures?kind=items']) {
    await expectWikiNotice(page, `/${lang}/wiki/${suffix}`, 400, notices[lang].invalid);
  }
});

test('wiki distinguishes an invalid entity key from an unavailable lookup', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  await expectWikiNotice(page, `/${lang}/wiki/creatures/unknown-id`, 503, notices[lang].unavailable);
  await expectWikiNotice(page, `/${lang}/wiki/creatures/${'x'.repeat(513)}`, 404, notices[lang].missing);
});
