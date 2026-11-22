import { createHash, randomUUID } from 'node:crypto';
import { pool } from '../helpers.mjs';
import { readFileSync, readdirSync, cpSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
 test, expect, binary, editPath, key, labels, link, locales, localized as t,
 mainForm, refuse, request, send, stamp, submitForm, title, visit,
} from '../content-helpers.mjs';

test('announcements draft, severity, expired window, approval and public action', async ({content}, {project}) => {
 const { anonymous, editor } = content;
 const page = editor.page;
 const lang = project.metadata.locale;
 const base = '/admin/announcements';
 const slug = key('announcement');
 const values = {
  title: `Announcement ${slug}`, body: 'English announcement body.',
  title_pl: `Ogłoszenie ${slug}`, body_pl: 'Polska treść ogłoszenia.',
  action_label: 'Read the rules', action_label_pl: 'Przeczytaj zasady', action_url: '/en/rules',
  starts_at: stamp(-20), translation_published_at: stamp(-19),
 };
 const ticker = locale => `/${locale}/announcements/ticker`;
 await refuse(content, base);
 await refuse(content, `${base}/create`);
 await refuse(content, base, 'POST', values);
 await visit(page, base);
 await title(page, t(lang, 'Ogłoszenia', 'Announcements'), lang);
 await link(page, `${base}/create`, t(lang, 'Utwórz', 'Create'));
 await expect(mainForm(page).getByLabel(t(lang, 'Publikacja', 'Publication')).locator('option'))
  .toHaveText([t(lang, 'Szkic', 'Draft'), t(lang, 'Opublikowane', 'Published')]);
 await expect(mainForm(page).getByLabel(t(lang, 'Ważność', 'Severity')).locator('option'))
  .toHaveText([t(lang, 'Informacja', 'Information'), t(lang, 'Przerwa techniczna', 'Maintenance'), t(lang, 'Ostrzeżenie', 'Warning')]);
 await expect(mainForm(page).getByRole('button')).toHaveText(t(lang, 'Zapisz', 'Save'));
 await labels(mainForm(page), lang, [
  ['starts_at', 'Początek (UTC)', 'Starts at (UTC)'],
  ['ends_at', 'Koniec (UTC, opcjonalny)', 'Ends at (UTC, optional)'],
  ['title', 'Tytuł (angielski)', 'Title (English)'],
  ['body', 'Treść (angielska)', 'Body (English)'],
  ['action_label', 'Etykieta akcji (angielska)', 'Action label (English)'],
  ['action_url', 'Ścieżka akcji', 'Action path'],
  ['title_pl', 'Tytuł (polski)', 'Title (Polish)'],
  ['body_pl', 'Treść (polska)', 'Body (Polish)'],
  ['action_label_pl', 'Etykieta akcji (polska)', 'Action label (Polish)'],
  ['translation_published_at', 'Data zatwierdzenia tłumaczenia (UTC)', 'Translation approved at (UTC)'],
 ]);
 await send(mainForm(page), { ...values, starts_at: 'bad date' }, 422);
 await send(mainForm(page), { ...values, action_url: 'https://example.invalid/rules' }, 422);
 await submitForm(mainForm(page), values);
 await expect(page).toHaveURL(/\/admin\/announcements\/[a-f0-9-]+\/edit$/);
 const edit = editPath(page);
 await refuse(content, edit);
 await refuse(content, edit, 'POST', values);
 await link(page, base, t(lang, 'Lista', 'List'));
 await link(page, edit, values.title);
 for (const locale of locales) {
  await visit(anonymous.page, ticker(locale));
  await expect(anonymous.page.locator('body')).not.toContainText(slug);
 }
 await send(mainForm(page), { ends_at: stamp(-30) }, 422);
 const stale = await mainForm(page).locator('[name="lock_version"]').inputValue();
 await submitForm(mainForm(page), { publication_state: 'published', ends_at: stamp(-10) });
 await send(mainForm(page), { lock_version: stale }, 409);
 await visit(anonymous.page, ticker('en'));
 await expect(anonymous.page.locator('body')).not.toContainText(slug);
 await submitForm(mainForm(page), { severity: 'warning', ends_at: '' });
 for (const locale of locales) {
  await visit(anonymous.page, ticker(locale));
  const heading = t(locale, `Ostrzeżenie: ${values.title_pl}`, `Warning: ${values.title}`);
  await expect(anonymous.page.locator('body')).toContainText(heading);
  await visit(anonymous.page, `/${locale}/today`);
  await title(anonymous.page, t(locale, 'Dziś', 'Today'), locale);
  await expect(anonymous.page.locator('main')).toContainText(heading);
  await visit(anonymous.page, `/${locale}/`);
  const banner = anonymous.page.getByRole('complementary', { name: t(locale, 'Ogłoszenia', 'Announcements') });
  await expect(banner).toContainText(heading);
  await expect(banner).toContainText(t(locale, values.body_pl, values.body));
  await banner.getByRole('link', { name: t(locale, values.action_label_pl, values.action_label) }).click();
  await expect(anonymous.page).toHaveURL(/\/en\/rules$/);
 }
 await visit(anonymous.page, `/announcements/ticker?lang=${lang}`);
 await expect(anonymous.page.locator('body')).toContainText(slug);
 await visit(anonymous.page, `/today?lang=${lang}`);
 await title(anonymous.page, t(lang, 'Dziś', 'Today'), lang);
 await submitForm(mainForm(page), { severity: 'maintenance', body: 'Updated English announcement.' });
 await visit(anonymous.page, ticker('pl'));
 await expect(anonymous.page.locator('body')).not.toContainText(slug);
 await visit(anonymous.page, ticker('en'));
 await expect(anonymous.page.locator('body')).toContainText(`Maintenance: ${values.title}`);
 await submitForm(mainForm(page), { body_pl: 'Zmienione ogłoszenie.', translation_published_at: stamp(-18) });
 await visit(anonymous.page, ticker('pl'));
 await expect(anonymous.page.locator('body')).toContainText('Zmienione ogłoszenie.');
 await submitForm(mainForm(page), { publication_state: 'draft' });
 for (const locale of locales) {
  await visit(anonymous.page, ticker(locale));
  await expect(anonymous.page.locator('body')).not.toContainText(slug);
 }

});

test('unprefixed news renders the announcement in the resolved page locale', async ({request}, {project}) => {
 const locale = project.metadata.locale;
 const id = randomUUID();
 const db = pool();
 const english = `English action ${id}`;
 const polish = `Polska akcja ${id}`;
 try {
  await db.query(`
   INSERT INTO announcements (
    id, publication_state, severity, starts_at, title, body, action_label, action_url,
    title_pl, body_pl, action_label_pl, translation_published_at, translation_source_updated_at,
    created_by, updated_by
   ) VALUES ($1, 'published', 'warning', now(), 'English title', 'English body', $2, '/en/rules',
    'Polski tytuł', 'Polska treść', $3, now(), now(), $1, $1)
  `, [id, english, polish]);
  for (const [path, preference, expected] of [
   ['/news', locale, locale],
   ['/news?lang=en', 'pl', 'en'],
   ['/pl/news?lang=en', 'en', 'pl'],
  ]) {
   const response = await request.get(path, { headers: { 'Accept-Language': preference } });
   expect(response.status()).toBe(200);
   expect(response.headers()['content-language']).toBe(expected);
   const html = await response.text();
   expect(html).toMatch(new RegExp(String.raw`<html\b[^>]*\slang="${expected}"`));
   expect(html).toContain(t(expected, polish, english));
   expect(html).not.toContain(t(expected, english, polish));
   expect(html).toMatch(new RegExp(String.raw`<header\b[^>]*id="portal-header"[\s\S]*?href="/${expected}/news"`));
  }
 } finally {
  await db.query('DELETE FROM announcements WHERE id=$1', [id]);
  await db.end();
 }
});

test('downloads CLI restricts publication and anonymous origin serves signed bytes', async ({content}) => {
 const { anonymous, editor, admin, player, support } = content;
 for (const channel of ['stable', 'beta']) {
  const dir = fileURLToPath(new URL(`../../crates/downloads/tests/fixtures/origin-v1/${channel}/`, import.meta.url));
  const root = `${dir}metadata/1.root.json`;
  const command = (accountId, bundle = dir) =>
   ['downloads-publish', '--channel', channel, '--actor', accountId, '--dir', bundle];
  const env = { [`OTERYN_DOWNLOADS_ROOT_${channel.toUpperCase()}`]: root };
  const base = `/updates/${channel}`;
  const policy = readFileSync(`${dir}policy-v1.json`);
  const hash = createHash('sha256').update(policy).digest('hex');
  const metadata = `${base}/metadata/timestamp.json`;
  for (const subject of [anonymous, player, support, editor, admin]) {
   await request(subject.page, metadata, 'POST', 405);
  }
  const unknown = '019a0000-0000-7000-8000-00000000dead';
  for (const accountId of [player.accountId, support.accountId, editor.accountId, unknown]) {
   expect(binary(command(accountId), env)).toEqual({ status: 1, refusal: 'publication denied' });
  }
  expect(binary(command('invalid'), env)).toEqual({ status: 1, refusal: 'invalid actor UUID' });
  const tampered = mkdtempSync(join(tmpdir(), 'content-downloads-'));
  try {
   cpSync(dir, tampered, { recursive: true });
   const timestamp = join(tampered, 'metadata', 'timestamp.json');
   const signed = JSON.parse(readFileSync(timestamp, 'utf8'));
   signed.signatures[0].sig = '00'.repeat(signed.signatures[0].sig.length / 2);
   writeFileSync(timestamp, JSON.stringify(signed));
   expect(binary(command(admin.accountId, tampered), env)).toEqual({
    status: 1, refusal: 'untrusted or inconsistent signed metadata',
   });
  } finally {
   rmSync(tampered, { recursive: true, force: true });
  }
  const policyUrl = `${base}/policy-v1.json`;
  await request(anonymous.page, `${base}/metadata/9.root.json`, 'GET', 404);
  await request(anonymous.page, '/updates/invalid/policy-v1.json', 'GET', 400);
  const previous = await anonymous.page.request.get(policyUrl);
  // The four browser projects share one disposable origin. An immutable signed
  // fixture can open a given channel only once.
  if (previous.status() === 404) {
   const published = binary(command(admin.accountId), env);
   expect(published.status, 'platform_admin publishes signed fixture').toBe(0);
   expect(published.stdout).toMatch(/^Published [a-f0-9-]+/);
  } else {
   expect(previous.status()).toBe(200);
   expect(await previous.body()).toEqual(policy);
  }
  expect(binary(command(admin.accountId), env)).toEqual({
   status: 1, refusal: 'publication would replay or rewrite trusted history',
  });
  const files = readdirSync(`${dir}metadata`);
  const mutable = 'no-store, no-transform';
  const immutable = 'public, max-age=31536000, immutable, no-transform';
  const checks = [
   [policyUrl, policy, mutable],
   [`${base}/targets/${hash}.channels/${channel}/policy-v1.json`, policy, immutable],
  ];
  for (const name of files) {
   const bytes = readFileSync(`${dir}metadata/${name}`);
   checks.push([`${base}/metadata/${name}`, bytes, name === 'timestamp.json' ? mutable : immutable]);
   if (name.startsWith('1.')) checks.push([`${base}/metadata/${name.slice(2)}`, bytes, mutable]);
  }
  for (const [path, bytes, cache] of checks) {
   const response = await request(anonymous.page, path, 'GET', 200);
   expect(response.headers()['cache-control']).toBe(cache);
   expect(response.headers()['content-type']).toBe('application/json');
   expect(await response.body()).toEqual(bytes);
  }
 }
});
