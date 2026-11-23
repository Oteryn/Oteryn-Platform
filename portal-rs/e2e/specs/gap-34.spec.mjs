import { readFile } from 'node:fs/promises';
import { test, expect } from '../fixtures.mjs';
import { submit } from '../helpers.mjs';
import { messages, seedEditor, visit } from '../public-depth-helpers.mjs';

test('events calendar follows publication and the viewer’s time zone', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const t = messages('templates/events', lang);
  const list = `/${lang}/events`;
  const calendar = `${list}/calendar/month`;
  const feed = `${list}/upcoming.ics`;
  const slug = `month-${crypto.randomUUID()}`;
  const detail = `${list}/${slug}`;
  const download = `${detail}/event.ics`;
  const title = { en: `Gathering, ${slug}`, pl: `Spotkanie, ${slug}` };
  const year = new Date().getUTCFullYear() + 1;
  const editor = await seedEditor(browser, info);
  const status = 'main form[action$="/status"]';
  let edit;
  async function transition(value) {
    await visit(editor.page, edit, 200);
    await editor.page.locator(`${status} select[name=status]`).selectOption(value);
    await submit(editor.page, status, { reason: 'Update calendar publication' });
    await expect(editor.page).toHaveURL(edit);
  }

  try {
    await visit(editor.page, '/admin/events/create', 200);
    await submit(editor.page, 'main form', {
      starts_at: `${year}-01-31T23:30`,
      ends_at: `${year}-02-01T01:00`,
      reason: 'Create calendar event',
      en_slug: slug, pl_slug: slug, en_title: title.en, pl_title: title.pl,
      en_summary: 'Gathering; programme.', pl_summary: 'Spotkanie; program.',
      en_body: 'Come join us.', pl_body: 'Dołącz do nas.',
    });
    await expect(editor.page).toHaveURL(/\/admin\/events\/[\da-f-]+\/edit$/);
    edit = editor.page.url();
    expect((await page.request.get(download)).status()).toBe(404);
    await transition('published');

    await visit(page, `/${lang}/`, 200, { lang });
    await page.locator(`footer.foot a[href="${list}"]`).click();
    await expect(page).toHaveURL(new RegExp(`${list}$`));
    const calendarLink = page.getByRole('link', { name: t.calendar, exact: true });
    await expect(calendarLink).toBeVisible();
    await calendarLink.click();
    await expect(page).toHaveURL(new RegExp(`${calendar}$`));
    await page.getByLabel(t.month, { exact: true }).fill(`${year}-02`);
    await page.getByRole('button', { name: t['show-month'] }).click();
    await expect(page.getByRole('heading', { name: `${t['month-february']} ${year}` })).toBeVisible();
    const day = page.locator(`[id="day-${year}-02-01"]`);
    const event = day.getByRole('link', { name: title[lang], exact: true });
    await expect(day.locator('time[datetime$="T23:30:00Z"]')).toContainText('23:30');

    const zone = page.getByLabel(t['time-zone'], { exact: true });
    await expect(zone).toHaveValue('UTC');
    await zone.selectOption('Europe/Warsaw');
    await page.getByRole('button', { name: t['save-time-zone'] }).click();
    await expect(zone).toHaveValue('Europe/Warsaw');
    await expect(day.locator('time[datetime$="T23:30:00Z"]')).toContainText('00:30');
    await page.getByRole('link', { name: t['previous-month'] }).click();
    await expect(page).toHaveURL(new RegExp(`month=${year}-01$`));
    await expect(page.locator(`main a[href="${detail}"]`)).toHaveCount(0);
    await page.getByRole('link', { name: t['next-month'] }).click();
    await expect(event).toBeVisible();
    await page.getByRole('table').getByRole('link', { name: '1', exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`#day-${year}-02-01$`));
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    await event.click();
    await expect(page.locator('main h1')).toHaveText(title[lang]);
    await expect(page.locator('main time').first()).toContainText('00:30');
    const exportLink = page.getByRole('link', { name: t['export-event'] });
    await expect(exportLink).toHaveAttribute('href', download);
    const [file] = await Promise.all([page.waitForEvent('download'), exportLink.click()]);
    expect(file.suggestedFilename()).toMatch(/\.ics$/);
    const single = await page.request.get(download);
    expect(single.status()).toBe(200);
    expect(single.headers()['content-type']).toContain('text/calendar');
    expect(single.headers()['content-disposition']).toMatch(/^attachment;.*\.ics/);
    const ics = (await readFile(await file.path(), 'utf8')).replace(/\r\n[ \t]/g, '');
    const uid = ics.match(/^UID:(.+)\r$/m)?.[1];
    expect(uid).toBeTruthy();
    expect(ics).toContain(`SUMMARY:${title[lang].replace(',', '\\,')}\r\n`);
    expect(ics).toContain(`DTSTART:${year}0131T233000Z\r\n`);
    expect(ics).toContain(`DTEND:${year}0201T010000Z\r\n`);

    await visit(page, list, 200, { lang });
    await expect(page.locator(`main a[href="${feed}"]`)).toHaveText(t['export-upcoming']);
    await expect(page.locator('main article').filter({ hasText: title[lang] }).locator('time').first()).toContainText('00:30');
    const upcoming = await page.request.get(feed);
    expect(upcoming.status()).toBe(200);
    expect(await upcoming.text()).toContain(`UID:${uid}\r\n`);

    await transition('archived');
    edit = undefined;
    expect((await page.request.get(download)).status()).toBe(404);
    expect(await (await page.request.get(feed)).text()).not.toContain(uid);
    await visit(page, `${calendar}?month=${year}-02`, 200);
    await expect(event).toHaveCount(0);
  } finally {
    try {
      if (edit) await transition('archived');
    } finally {
      await editor.context.close();
    }
  }
});

test('i18n-parity for events', () => {
  const en = messages('templates/events', 'en');
  const pl = messages('templates/events', 'pl');
  expect(Object.keys(pl).sort()).toEqual(Object.keys(en).sort());
  for (const key of ['calendar', 'time-zone', 'save-time-zone', 'export-event', 'export-upcoming']) {
    expect(en[key]).toBeTruthy();
    expect(pl[key]).toBeTruthy();
  }
});
