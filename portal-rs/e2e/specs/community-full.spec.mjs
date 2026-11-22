import { test, expect } from '../fixtures.mjs';
import { follow, open, strings, withEvents, withEditorial } from '../community-helpers.mjs';

test('editorial events order, detail links, pagination and empty page', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const path = `/${lang}/events`;
  const t = strings('templates/events', lang);
  const title = lang === 'pl' ? 'Spotkanie społeczności' : 'Community gathering';
  const summary = lang === 'pl' ? 'Spotkajmy się na placu.' : 'Meet at the plaza.';
  const body = lang === 'pl' ? 'Przynieś latarnię.' : 'Bring a lantern.';
  const articles = page.locator('main article');
  await withEvents(async remove => {
    for (const [alias, target] of [['/events', '/pl/events'], ['/events/community-e2e-1', '/pl/events/community-e2e-1']]) {
      const redirect = await page.request.get(alias, {maxRedirects: 0});
      expect(redirect.status()).toBe(303);
      expect(redirect.headers().location).toBe(target);
    }
    await open(page, path);
    await expect(page.locator('main h1')).toHaveText(t.events);
    await expect(articles.locator('h2')).toHaveText(Array.from({length: 200}, (_, i) => `${title} ${i + 1}`));
    await expect(articles.first()).toContainText(summary);
    await expect(articles.first()).not.toContainText(body);
    await expect(articles.first().locator('time')).toHaveCount(2);

    const links = await articles.getByRole('link').evaluateAll(nodes => nodes.map(node => node.getAttribute('href')));
    for (let offset = 0; offset < links.length; offset += 8) {
      await Promise.all(links.slice(offset, offset + 8).map(async (link, i) => {
        expect(link).toBe(`${path}/community-e2e-${offset + i + 1}`);
        const detail = await page.request.get(link);
        expect(detail.status(), link).toBe(200);
        await detail.dispose();
      }));
    }
    const first = `${path}/community-e2e-1`;
    await follow(page, articles.first().getByRole('link'), first);
    await expect(page.locator('main h1')).toHaveText(`${title} 1`);
    await expect(articles).toContainText(body);
    await expect(articles.getByRole('link')).toHaveAttribute('href', first);

    await open(page, path);
    const next = page.locator('main a[rel=next]');
    await expect(next).toHaveText(lang === 'pl' ? 'Następna strona' : 'Next page');
    await follow(page, next, path, 200, {page: '1'});
    await expect(articles.locator('h2')).toHaveText([`${title} 201`]);
    await follow(page, articles.getByRole('link'), `${path}/community-e2e-201`);
    await expect(articles).toContainText(body);

    await open(page, `/${lang}/today`);
    const digest = page.locator('main section[aria-labelledby=events]');
    await expect(digest.getByRole('heading', {level: 2})).toHaveText(t.events);
    await expect(digest.getByRole('heading', {level: 3})).toHaveText(`${title} 1`);
    await expect(digest.getByText(summary)).toBeVisible();
    const view = digest.getByRole('link', {name: lang === 'pl' ? 'Zobacz' : 'View'});
    await follow(page, view, first);
    await expect(articles).toContainText(body);

    await remove(201);
    await open(page, path);
    await expect(articles).toHaveCount(200);
    await expect(next).toHaveCount(0);
    await open(page, `${path}?page=1`);
    await expect(articles).toHaveCount(0);
    await expect(page.locator('main')).toContainText(t.empty);
    await open(page, `${path}/community-e2e-201`, 404);
    await open(page, `${path}/community-e2e-202`, 404);
    await open(page, `${path}?page=bad`, 400);
  });
});

test('portal search editorial filters, pagination, cold sources and invalid input', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const path = `/${lang}/search`;
  const t = strings('crates/web/src/search', lang);
  const main = page.locator('main');
  const q = main.getByLabel(t.query, {exact: true});
  const kind = main.getByLabel(t.kind, {exact: true});
  const phrase = lang === 'pl' ? 'Spotkanie społeczności' : 'Community gathering';
  const articles = main.locator('article');

  await withEvents(async () => {
    await open(page, '/search');
    await expect(main.getByRole('heading', {level: 1})).toHaveText(t.title);
    await open(page, path);
    await expect(main.getByRole('heading', {level: 1})).toHaveText(t.title);
    await expect(kind.locator('option')).toHaveText(
      ['all', 'news', 'pages', 'events', 'wiki', 'world', 'characters'].map(key => t[key])
    );
    await expect(main.getByText(t.empty, {exact: true})).toBeVisible();

    for (const filter of ['', 'events', 'news', 'pages', 'wiki', 'world', 'characters']) {
      const cold = ['world', 'characters'].includes(filter);
      await q.fill(phrase);
      await kind.selectOption(filter);
      await follow(page, main.getByRole('button', {name: t.search, exact: true}), path,
        cold ? 503 : 200, {q: phrase, kind: filter});
      await expect(q).toHaveValue(phrase);
      await expect(kind).toHaveValue(filter);

      if (cold) {
        await expect(main.getByRole('status')).toHaveText(t.unavailable);
        await expect(articles).toHaveCount(0);
      } else if (filter === '' || filter === 'events') {
        await expect(articles).toHaveCount(20);
        await expect(articles.first().locator('p')).toHaveText(t.events);
        const firstPage = await articles.getByRole('link').allTextContents();
        await follow(page, main.getByRole('button', {name: t.next}), path, 200,
          {q: phrase, kind: filter, page: '2'});
        await expect(articles).toHaveCount(20);
        expect(await articles.getByRole('link').allTextContents()).not.toEqual(firstPage);
        const link = articles.first().getByRole('link');
        const label = await link.innerText();
        await follow(page, link, await link.getAttribute('href'));
        await expect(main.getByRole('heading', {level: 1})).toHaveText(label);
        await open(page, path);
      } else {
        await expect(main.getByText(t.empty, {exact: true})).toBeVisible();
        await expect(articles).toHaveCount(0);
      }
    }
    await q.fill('x');
    await follow(page, main.getByRole('button', {name: t.search, exact: true}), path, 400, {q: 'x'});
    await expect(main.getByText(t.invalid, {exact: true})).toBeVisible();
  });
});

test('editorial search opens localized Wiki, news and page details', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const path = `/${lang}/search`;
  const t = strings('crates/web/src/search', lang);
  const main = page.locator('main');
  const title = lang === 'pl' ? 'Warsztat latarni' : 'Lantern workshop';
  const summary = lang === 'pl' ? 'Poradnik o latarniach.' : 'A guide to lanterns.';
  const heading = lang === 'pl' ? 'Montaż' : 'Assembly';
  const paragraph = lang === 'pl' ? 'Załóż knot.' : 'Fit the wick.';
  await withEditorial(async () => {
    await open(page, path);
    await main.getByLabel(t.query).fill(title);
    await main.getByLabel(t.kind).selectOption('wiki');
    await follow(page, main.getByRole('button', {name: t.search, exact: true}), path, 200, {q: title, kind: 'wiki'});
    await expect(main.locator('article')).toHaveCount(1);
    await expect(main.locator('article p')).toHaveText(t.wiki);
    const link = main.getByRole('link', {name: title});
    await follow(page, link, `/${lang}/wiki/articles/community-e2e-guide`);
    await expect(main.getByRole('heading', {level: 1})).toHaveText(title);
    await expect(main.getByText(summary)).toBeVisible();
    await expect(main.getByRole('heading', {level: 2, name: heading})).toBeVisible();
    await expect(main.getByText(paragraph)).toBeVisible();
    for (const [kind, headline, content, target] of [
      ['news', lang === 'pl' ? 'Festiwal latarni' : 'Lantern festival',
        lang === 'pl' ? 'Bramy otworzą się o zmierzchu.' : 'The gates open at dusk.',
        `/${lang}/news/community-e2e-news`],
      ['pages', lang === 'pl' ? 'Droga na plac' : 'Plaza directions',
        lang === 'pl' ? 'Idź drogą na wschód.' : 'Follow the eastern road.',
        `/${lang}/pages/community-e2e-page`],
    ]) {
      await open(page, path);
      await main.getByLabel(t.query).fill(headline);
      await main.getByLabel(t.kind).selectOption(kind);
      await follow(page, main.getByRole('button', {name: t.search, exact: true}), path, 200,
        {q: headline, kind});
      await expect(main.locator('article')).toHaveCount(1);
      await expect(main.locator('article p')).toHaveText(t[kind]);
      await follow(page, main.getByRole('link', {name: headline}), target);
      await expect(main.getByRole('heading', {level: 1})).toHaveText(headline);
      await expect(main.getByText(content, {exact: true})).toBeVisible();
    }
  });
});
