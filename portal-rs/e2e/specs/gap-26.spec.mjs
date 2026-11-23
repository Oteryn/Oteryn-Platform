import { readFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { submit } from '../helpers.mjs';
import { seededAccount, seededLogin, seededStaff, unique } from '../staff-rbac-helpers.mjs';

test('ticket screenshots are shared by the player and support', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const labels = lang === 'pl'
    ? { upload: 'Dodaj zrzut ekranu', screenshots: 'Zrzuty ekranu', image: 'Zrzut ekranu ze zgłoszenia' }
    : { upload: 'Attach screenshot', screenshots: 'Screenshots', image: 'Ticket screenshot' };
  await seededLogin(page, lang, await seededAccount(lang));
  await page.goto(`/${lang}/support/tickets/create`);
  const ticketsLink = `a[href="/${lang}/support/tickets"]`;
  expect(await page.locator(`header ${ticketsLink}, #podglad-konta ${ticketsLink}`).count()).toBeGreaterThan(0);
  const subject = `Screenshot ${unique()}${'w'.repeat(80)}`;
  await submit(page, 'main form', { subject, body: 'Screenshot for the support conversation.' });
  await expect(page).toHaveURL(/support\/tickets\/[a-f0-9-]+$/);
  const ticket = new URL(page.url()).pathname;
  const origin = new URL(page.url()).origin;

  const screenshot = async (actor, color) => ({
    name: 'screenshot.png',
    mimeType: 'image/png',
    buffer: Buffer.from(await actor.evaluate(color => {
      const canvas = document.createElement('canvas');
      canvas.width = 640;
      canvas.height = 360;
      const context = canvas.getContext('2d');
      context.fillStyle = color;
      context.fillRect(0, 0, canvas.width, canvas.height);
      return canvas.toDataURL('image/png').split(',')[1];
    }, color), 'base64'),
  });

  const upload = async (actor, file) => {
    const form = actor.locator('form[action$="/attachments"]');
    await form.locator('input[type=file]').setInputFiles(file);
    await form.getByRole('button', { name: labels.upload }).click();
  };
  const rendered = async (actor, count) => {
    const images = actor.getByRole('img', { name: labels.image });
    await expect(actor.getByRole('banner')).toBeVisible();
    await expect(images).toHaveCount(count);
    await expect.poll(() => images.evaluateAll(elements => elements.every(
      image => image.complete && image.naturalWidth === 640
    ))).toBe(true);
    await expect.poll(() => actor.evaluate(async () => {
      await document.fonts.ready;
      return document.documentElement.scrollWidth - innerWidth;
    }), { message: 'ticket page fits the viewport' }).toBeLessThanOrEqual(0);
    const source = await images.last().getAttribute('src');
    const response = await actor.request.get(source);
    expect(response.status()).toBe(200);
    expect(response.headers()['content-type']).toBe('image/png');
    expect(response.headers()['cache-control']).toContain('no-store');
    expect(response.headers()['x-content-type-options']).toBe('nosniff');
    return source;
  };

  const playerFile = await screenshot(page, 'blue');
  const form = page.locator('form[action$="/attachments"]');
  await expect(form).toBeVisible();
  const csrf = await form.locator('input[name=_csrf]').inputValue();
  const requestKey = await form.locator('input[name=request_key]').inputValue();
  const post = (actor, multipart) => actor.request.post(`${ticket}/attachments`, {
    multipart, headers: { origin }, maxRedirects: 0,
  });
  expect((await post(page, { image: playerFile })).status()).toBe(403);
  expect((await post(page, { _csrf: csrf, image: playerFile })).status()).toBe(422);
  expect((await post(page, {
    _csrf: csrf, request_key: requestKey,
    image: { ...playerFile, buffer: Buffer.from('invalid image') },
  })).status()).toBe(422);
  expect((await post(page, {
    _csrf: csrf, request_key: requestKey,
    image: { ...playerFile, buffer: Buffer.alloc(8 * 1024 * 1024 + 1) },
  })).status()).toBe(413);
  await upload(page, playerFile);
  const imagePath = await rendered(page, 1);
  expect((await post(page, { _csrf: csrf, request_key: requestKey, image: playerFile })).status()).toBe(302);
  expect((await post(page, {
    _csrf: csrf, request_key: requestKey, image: await screenshot(page, 'green'),
  })).status()).toBe(409);
  await page.reload();
  expect(await rendered(page, 1)).toBe(imagePath);

  const team = await browser.newContext({
    baseURL: origin, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport,
  });
  const stranger = await browser.newContext({ baseURL: origin, ignoreHTTPSErrors: true });
  try {
    const staffPage = await team.newPage();
    await seededStaff(staffPage, lang, 'support');
    expect((await staffPage.goto('/admin/support/tickets')).status()).toBe(200);
    await staffPage.getByRole('link', { name: subject, exact: true }).click();
    await staffPage.getByRole('link', { name: labels.screenshots }).click();
    await rendered(staffPage, 1);
    const staffFile = await screenshot(staffPage, 'red');
    const staffForm = staffPage.locator('form[action$="/attachments"]');
    const staffAction = await staffForm.getAttribute('action');
    expect((await staffPage.request.post(staffAction, {
      multipart: { image: staffFile }, headers: { origin }, maxRedirects: 0,
    })).status()).toBe(403);
    await upload(staffPage, staffFile);
    const staffPath = await rendered(staffPage, 2);

    await page.reload();
    await rendered(page, 2);
    const other = await stranger.newPage();
    await seededLogin(other, lang, await seededAccount(lang));
    expect((await other.request.get(imagePath)).status()).toBe(404);
    expect((await other.request.get(staffPath, { maxRedirects: 0 })).status()).toBe(403);
    await other.goto(`/${lang}/support/tickets/create`);
    const otherCsrf = await other.locator('input[name=_csrf]').first().inputValue();
    expect((await post(other, { _csrf: otherCsrf, request_key: requestKey, image: playerFile })).status()).toBe(404);
  } finally {
    await stranger.close();
    await team.close();
  }
});

test('screenshot i18n-parity', () => {
  const keys = (path) => [...readFileSync(new URL(path, import.meta.url), 'utf8')
    .matchAll(/^([\w-]+)\s*=\s*\S/gm)].map((match) => match[1]).sort();
  for (const catalogue of [
    (lang) => `../../crates/support/src/tickets/${lang}.ftl`,
    (lang) => `../../crates/admin/src/support_${lang}.ftl`,
  ]) {
    const pl = keys(catalogue('pl'));
    expect(pl).toContain('screenshots');
    expect(pl).toEqual(keys(catalogue('en')));
  }
});
