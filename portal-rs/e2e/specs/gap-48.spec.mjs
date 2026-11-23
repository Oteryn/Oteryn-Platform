import { readFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { login } from '../helpers.mjs';
import { seedPlayer, visit } from '../public-depth-helpers.mjs';
import { assertShell } from '../shell.mjs';

test('a player records a private weekly note and downloads it while reports are unavailable', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const report = `/${lang}/account/journal/reports`;
  const editor = `${report}/weekly-note`;
  const download = `${editor}/export`;
  const body = '\nRemember to review the <route> next week.\nCheck the notes before playing.';

  await login(page, lang, await seedPlayer(lang));
  await visit(page, report, 503);
  await page.locator(`main a[href^="reports/weekly-note"]`).click();
  await expect(page).toHaveURL(new RegExp(`${editor}\\?week=\\d+$`));
  const form = page.locator('main form');
  await expect(page.locator('header#portal-header')).toHaveCount(1);
  await expect(page.locator('footer.foot')).toHaveCount(1);
  await expect(page.getByRole('main')).toHaveCount(1);
  await expect(page.getByRole('heading', { level: 1 })).toHaveCount(1);
  await expect(page.locator('header#portal-header a, #podglad-konta a').filter({
    hasText: lang === 'pl' ? 'Prywatne notatki tygodniowe' : 'Private weekly notes',
  }).first()).toHaveAttribute('aria-current', 'page');
  const overLimit = 'ż'.repeat(2001);
  await form.locator('textarea').fill(overLimit);
  const [invalid] = await Promise.all([
    page.waitForResponse(response => response.request().method() === 'POST' && response.url().includes(editor)),
    form.locator('button[type=submit]').click(),
  ]);
  expect(invalid.status()).toBe(422);
  await expect(page.getByRole('alert')).toHaveText(lang === 'pl'
    ? 'Wpisz zwykły tekst o długości do 4000 bajtów.'
    : 'Use up to 4000 bytes of plain text.');
  await expect(form.locator('textarea')).toHaveValue(overLimit);
  await form.locator('textarea').fill(body);
  await form.locator('button[type=submit]').click();
  await expect(form.locator('textarea')).toHaveValue(body);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);

  const downloadLink = page.locator('main a[href="weekly-note/export?format=json"]');
  const [file] = await Promise.all([page.waitForEvent('download'), downloadLink.click()]);
  expect(file.suggestedFilename()).toBe('weekly-journal-notes.json');
  const exported = await page.request.get(download);
  expect(exported.status()).toBe(200);
  expect(exported.headers()['cache-control']).toContain('no-store');
  const [note] = await exported.json();
  expect(note.body).toBe(body);
  expect((await page.request.get(`${download}?format=csv`)).status()).toBe(422);

  const other = await browser.newContext({
    baseURL: new URL(page.url()).origin,
    locale: lang,
    viewport: info.project.use.viewport,
    ignoreHTTPSErrors: true,
  });
  try {
    const stranger = await other.newPage();
    await login(stranger, lang, await seedPlayer(lang));
    expect((await stranger.request.get(`${download}?id=${note.id}`)).status()).toBe(404);
    expect(await (await stranger.request.get(download)).json()).toEqual([]);
  } finally {
    await other.close();
  }

  await form.locator('textarea').clear();
  await form.locator('button[type=submit]').click();
  await expect(form.locator('textarea')).toHaveValue('');
  expect((await page.request.get(`${download}?id=${note.id}`)).status()).toBe(404);
});

test('Atlas keeps participation controls usable when cohort evidence is unavailable', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const places = `/${lang}/atlas/places`;
  await login(page, lang, await seedPlayer(lang));
  await visit(page, places, 503);
  await assertShell(page, { status: 503 });
  const form = page.locator('main form');
  for (const participating of [true, false]) {
    await form.locator(`input[name=participating][value="${participating}"]`).check();
    await form.locator('button[type=submit]').click();
    await expect(form.locator(`input[name=participating][value="${participating}"]`)).toBeChecked();
    const state = await page.request.get(`${places}/participation`);
    expect(state.status()).toBe(200);
    expect(await state.json()).toEqual({ participating });
    await expect(page.locator('main .places li')).toHaveCount(0);
  }
});

test('i18n-parity for journal notes and hunting places', () => {
  for (const source of [
    'crates/playerdata/src/reports_{lang}.ftl',
    'crates/playerdata/src/places/{lang}.ftl',
    'crates/ui/i18n/{lang}.ftl',
  ]) {
    const keys = lang => [...readFileSync(new URL(`../../${source.replace('{lang}', lang)}`, import.meta.url), 'utf8')
      .matchAll(/^([\w-]+)\s*=/gm)].map(match => match[1]).sort();
    const english = keys('en');
    expect(english.length).toBeGreaterThan(0);
    expect(new Set(english).size).toBe(english.length);
    expect(keys('pl')).toEqual(english);
  }
});
