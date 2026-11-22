import { test, expect } from '../fixtures.mjs';
import { pool } from '../helpers.mjs';
import { follow, open, strings } from '../community-helpers.mjs';

test('character statistics submits the target level and bounded dates', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const path = `/${lang}/stats/characters/019a0000-0000-7000-8000-000000000001/experience`;
  const t = strings('templates/stats/character', lang);
  await open(page, path, 503);
  const main = page.locator('main');
  const start = '2026-11-01';
  const end = '2026-11-02';
  await main.getByLabel(t.start, {exact: true}).fill(start);
  await main.getByLabel(t.end, {exact: true}).fill(end);
  const target = main.getByLabel(t.target, {exact: true});
  await target.fill('0');
  expect(await target.evaluate(input => input.validity.rangeUnderflow)).toBe(true);
  await target.fill('100');
  await follow(page, main.getByRole('button', {name: t.apply, exact: true}),
    path, 503, {start, end, target: '100'});
  await expect(main.getByRole('status')).toHaveText(t.unavailable);
  await expect(main.locator('article, figure')).toHaveCount(0);

  await open(page, `${path}?start=${start}&end=${end}&target=0`, 400);
  await expect(main.getByRole('status')).toHaveText(t.invalid);
});

test('Today digest and ticker link to localized public events', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const target = `/${lang}/events`;
  const db = pool();
  let announcement;
  try {
    announcement = (await db.query(`
      INSERT INTO announcements
      (publication_state,severity,starts_at,ends_at,title,body,action_label,action_url,
       title_pl,body_pl,action_label_pl,translation_published_at,translation_source_updated_at,created_by,updated_by)
      VALUES ('published','warning',now()-interval '1 hour',now()+interval '1 hour',
       'Plaza notice','The north gate is open.','View events',$1,
       'Ogłoszenie na placu','Brama północna jest otwarta.','Zobacz wydarzenia',now(),now(),uuidv7(),uuidv7())
      RETURNING id
    `, [target])).rows[0].id;

    const title = lang === 'pl' ? 'Ostrzeżenie: Ogłoszenie na placu' : 'Warning: Plaza notice';
    const body = lang === 'pl' ? 'Brama północna jest otwarta.' : 'The north gate is open.';
    const action = lang === 'pl' ? 'Zobacz wydarzenia' : 'View events';
    await open(page, `/${lang}/today`);
    const section = page.locator('main section[aria-labelledby=announcements]');
    await expect(section.getByRole('heading', {level: 2})).toHaveText(lang === 'pl' ? 'Ogłoszenia' : 'Announcements');
    await expect(section.getByRole('heading', {name: title, exact: true})).toBeVisible();
    await expect(section.getByText(body, {exact: true})).toBeVisible();
    await follow(page, section.getByRole('link', {name: action, exact: true}), target);
    await expect(page.locator('main h1')).toHaveText(strings('templates/events', lang).events);

    for (const tickerPath of [`/announcements/ticker?lang=${lang}`, `/${lang}/announcements/ticker`]) {
      const ticker = await open(page, tickerPath);
      expect(ticker.headers()['cache-control']).toContain('no-store');
      const notice = page.getByRole('complementary');
      await expect(notice.getByRole('heading', {name: title, exact: true})).toBeVisible();
      await expect(notice.getByText(body, {exact: true})).toBeVisible();
      await follow(page, notice.getByRole('link', {name: action, exact: true}), target);
    }
  } finally {
    try {
      if (announcement) await db.query('DELETE FROM announcements WHERE id=$1', [announcement]);
    } finally {
      await db.end();
    }
  }
});
