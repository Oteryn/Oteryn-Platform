import {randomUUID} from 'node:crypto';
import {readFileSync} from 'node:fs';
import {test, expect} from '../fixtures.mjs';
import {submit} from '../helpers.mjs';
import {seededStaff} from '../staff-rbac-helpers.mjs';

const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAHklEQVR4nGNQzrP5T03MMGrgqIGjBo4aOGrgSDUQANpkzt/P0xIDAAAAAElFTkSuQmCC', 'base64');

test('i18n-parity for media library labels', () => {
  const keys = lang => [...readFileSync(new URL(
    `../../crates/content/locales/${lang}.ftl`, import.meta.url,
  ), 'utf8').matchAll(/^(media-[\w-]+) = \S/gm)].map(([, key]) => key).sort();
  const english = keys('en');
  expect(english.length).toBeGreaterThan(0);
  expect(keys('pl')).toEqual(english);
  expect(new Set(english).size).toBe(english.length);
});

test('reuse a library image in bilingual news and replace it before removal', async ({page, browser}, info) => {
  const lang = info.project.metadata.locale;
  await seededStaff(page, lang, 'platform_admin');
  const key = `media-${randomUUID()}`;
  const original = `${key} original`;
  const replacement = `${key} replacement`;
  const library = `/admin/media?lang=${lang}`;

  await page.goto(`/admin/news?lang=${lang}`);
  await page.locator(`main a[href="${library}"]`).click();
  await expect(page.locator('html')).toHaveAttribute('lang', lang);

  async function upload(alt, filename) {
    const form = 'main form[enctype="multipart/form-data"]';
    await page.locator(`${form} input[type=file]`).setInputFiles({name: filename, mimeType: 'image/png', buffer: png});
    await submit(page, form, {alt_text: alt});
    const image = page.getByRole('img', {name: alt, exact: true});
    await expect(image).toBeVisible();
    return (await image.getAttribute('src')).split('/')[3];
  }

  const originalId = await upload(original, `${key}.png`);
  const replacementId = await upload(replacement, 'replacement.png');
  await page.locator('input[name=q]').fill(`${key}.png`);
  await page.locator('form[role=search] button').click();
  await expect(page.getByRole('img', {name: original, exact: true})).toBeVisible();
  await expect(page.getByRole('img', {name: replacement, exact: true})).toHaveCount(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);

  const reader = await browser.newPage({
    baseURL: new URL(page.url()).origin,
    ignoreHTTPSErrors: true,
    viewport: info.project.use.viewport,
    locale: lang,
  });
  try {
    expect((await reader.request.get(`/news/media/${originalId}`)).status()).toBe(404);
    await page.locator(`main a[href="/admin/news/create?lang=${lang}&media_id=${originalId}"]`).click();
    await expect(page.locator('input[name=media_id]')).toHaveValue(originalId);
    const publication = new Date(Date.now() - 60_000).toISOString();
    await submit(page, 'main form', {
      slug: key,
      title: key,
      body: 'Illustrated news',
      title_pl: key,
      body_pl: 'Wiadomości z ilustracją',
      published_at: publication,
      translation_published_at: publication,
    });
    await expect(page).toHaveURL(/\/admin\/news\/[a-f0-9-]+\/edit/);
    for (const locale of ['pl', 'en']) {
      expect((await reader.goto(`/${locale}/news/${key}`)).status()).toBe(200);
      const image = reader.getByRole('img', {name: original, exact: true});
      await expect(image).toBeVisible();
      await expect(image).toHaveJSProperty('complete', true);
      expect(await image.evaluate(img => img.naturalWidth)).toBeGreaterThan(0);
      expect(await reader.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    }

    await page.goto(`${library}&q=${key}`);
    const asset = page.locator('main article').filter({has: page.getByRole('img', {name: original, exact: true})});
    await expect(asset.getByRole('link', {name: key, exact: true})).toBeVisible();
    const form = asset.locator('form[action$="/delete"]');
    const csrf = await form.locator('[name=_csrf]').inputValue();
    const action = await form.getAttribute('action');
    expect((await page.request.post(action, {
      headers: {origin: new URL(page.url()).origin},
      form: {_csrf: csrf},
      maxRedirects: 0,
    })).status()).toBe(422);
    await submit(page, `main form[action="${action}"]`, {replacement_id: replacementId});
    await expect(page.getByRole('img', {name: original, exact: true})).toHaveCount(0);
    for (const locale of ['pl', 'en']) {
      expect((await reader.goto(`/${locale}/news/${key}`)).status()).toBe(200);
      await expect(reader.getByRole('img', {name: replacement, exact: true})).toBeVisible();
    }
    const replacementCard = page.locator('main article').filter({has: page.getByRole('img', {name: replacement, exact: true})});
    await replacementCard.locator('summary').click();
    const updated = `${key} updated description`;
    await submit(page, `main form[action="/admin/media/${replacementId}"]`, {alt_text: updated});
    await reader.reload();
    await expect(reader.getByRole('img', {name: updated, exact: true})).toBeVisible();
    expect((await reader.request.get(`/news/media/${originalId}`)).status()).toBe(404);
  } finally {
    await reader.close();
  }
});
