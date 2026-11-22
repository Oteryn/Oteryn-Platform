import { createHash } from 'node:crypto';
import { query } from '../staff-rbac-helpers.mjs';
import {
 test, expect, formFor, key, labels, localized as t, png,
 refuse, request, send, submitForm, title, visit,
} from '../content-helpers.mjs';

test('media rejects invalid uploads and serves normalized images until deletion', async ({content}, {project}) => {
 const { editor } = content;
 const page = editor.page;
 const lang = project.metadata.locale;
 const base = '/admin/media';
 const alt = key('editorial-image');
 await refuse(content, base);
 await refuse(content, base, 'POST', {}, { alt_text: alt, image: png });
 await visit(page, `${base}?lang=${lang}`);
 await title(page, 'Media', lang);
 const upload = formFor(page, base);
 await labels(upload, lang, [
  ['alt_text', 'Tekst alternatywny', 'Alternative text'],
  ['image', 'Obraz (JPEG, PNG, WebP; maks. 8 MiB)', 'Image (JPEG, PNG, WebP; max 8 MiB)'],
 ]);
 const file = upload.locator('[name="image"]');
 await file.setInputFiles({ ...png, buffer: Buffer.from('invalid PNG bytes') });
 const button = `button:has-text("${t(lang, 'Prześlij', 'Upload')}")`;
 const bad = page.waitForResponse(response => response.request().method() === 'POST'
  && new URL(response.url()).pathname === base && response.status() === 422);
 const failedPage = page.waitForEvent('framenavigated', frame => frame === page.mainFrame());
 await submitForm(upload, { alt_text: alt }, button);
 expect((await bad).status()).toBe(422);
 await failedPage;
 await visit(page, `${base}?lang=${lang}`);
 await file.setInputFiles(png);
 await submitForm(upload, { alt_text: alt }, button);
 const thumbnail = page.getByRole('img', { name: alt, exact: true });
 await expect(thumbnail).toBeVisible();
 await title(page, 'Media', lang);
 await expect(upload.locator('button')).toHaveText(t(lang, 'Prześlij', 'Upload'));
 const src = await thumbnail.getAttribute('src');
 const row = src.replace(/\/thumbnail$/, '');
 for (const path of [src, `${row}/content`]) {
  await refuse(content, path);
  const response = await request(page, path, 'GET', 200);
  expect(response.headers()['content-type']).toBe('image/png');
  expect((await response.body()).subarray(0, 8)).toEqual(png.buffer.subarray(0, 8));
 }
 await expect.poll(() => thumbnail.evaluate(image => image.naturalWidth)).toBe(20);
 await refuse(content, `${row}/delete`, 'POST');
 await refuse(content, row, 'DELETE');
 const remove = formFor(page, `${row}/delete`);
 await expect(remove.getByRole('button')).toHaveText(t(lang, 'Usuń', 'Delete'));
 const id = row.split('/').at(-1);
 await query('INSERT INTO media_references(media_id,consumer) VALUES($1,$2)', [id, alt]);
 try {
  await send(remove, {}, 422);
  await expect(thumbnail).toBeVisible();
 } finally {
  await query('DELETE FROM media_references WHERE media_id=$1 AND consumer=$2', [id, alt]);
 }
 await submitForm(remove);
 await expect(thumbnail).toHaveCount(0);
 await request(page, src, 'GET', 404);
 await request(page, `${row}/content`, 'GET', 404);
 await request(page, row, 'DELETE', 403);
 for (const [extension, mimeType] of [['jpg', 'image/jpeg'], ['webp', 'image/webp']]) {
  const fixture = await page.evaluate(type => {
   const canvas = document.createElement('canvas');
   canvas.width = 400;
   canvas.height = 200;
   const drawing = canvas.getContext('2d');
   drawing.fillStyle = 'navy';
   drawing.fillRect(0, 0, 400, 200);
   return canvas.toDataURL(type);
  }, mimeType);
  expect(fixture).toMatch(new RegExp(`^data:${mimeType};base64,`));
  await file.setInputFiles({
   name: `wide.${extension}`, mimeType, buffer: Buffer.from(fixture.split(',')[1], 'base64'),
  });
  await submitForm(upload, { alt_text: alt });
  await expect(thumbnail).toBeVisible();
  await expect.poll(() => thumbnail.evaluate(image => image.naturalWidth)).toBe(320);
  const preview = await thumbnail.getAttribute('src');
  const asset = preview.replace(/\/thumbnail$/, '');
  const original = await request(page, `${asset}/content`, 'GET', 200);
  const resized = await request(page, preview, 'GET', 200);
  expect(original.headers()['content-type']).toBe(mimeType);
  expect(resized.headers()['content-type']).toBe(mimeType);
  expect(await resized.body()).not.toEqual(await original.body());
  const deletion = formFor(page, `${asset}/delete`);
  await send(deletion, {}, 302, 'DELETE', asset);
  await send(deletion, {}, 404, 'DELETE', asset);
  await request(page, `${asset}/content`, 'GET', 404);
  await visit(page, base);
  await expect(thumbnail).toHaveCount(0);
 }
});

test('editorial library pagination reaches entries beyond each first-page limit', async ({content}, {project}) => {
 const { anonymous, editor } = content;
 const page = editor.page;
 const lang = project.metadata.locale;
 const prefix = key('paging');
 const args = [prefix + '-'];
 const digest = createHash('sha256').update(png.buffer).digest();
 // Seed only the volume needed to reach the navigation controls. All
 // publication mutations have their own form-driven lifecycle tests.
 try {
  const news = await query(`
   INSERT INTO news(slug,title,body,published_at,title_pl,body_pl,translation_published_at,
                    translation_source_updated_at,updated_at)
   SELECT $1||n,'News '||n,'English.',now(),'Aktualność '||n,'Treść.',now(),now(),now()
   FROM generate_series(1,26) n RETURNING id,slug`, args);
  await query(`
   WITH created AS (
    INSERT INTO pages(id,slug) SELECT uuidv7(),$1||n FROM generate_series(1,501) n RETURNING id,slug
   )
   INSERT INTO editorial_pages(page_id,locale,title,body)
   SELECT id,'en',slug,'Page text' FROM created`, args);
  const announcements = await query(`
   INSERT INTO announcements(severity,starts_at,title,body,created_by,updated_by)
   SELECT 'info',now(),$1||n,'Announcement text',$2,$2
   FROM generate_series(1,501) n RETURNING id`, [...args, editor.accountId]);
  const media = await query(`
   INSERT INTO media(id,alt_text,mime_type,width,height,content,sha256,thumbnail,thumbnail_sha256)
   SELECT uuidv7(),$1||n,'image/png',20,20,$2,$3,$2,$3
   FROM generate_series(1,101) n RETURNING id`, [...args, png.buffer, digest]);
  const events = await query(`
   WITH created AS (
    INSERT INTO events(status,starts_at,ends_at,created_by,updated_by,published_by)
    SELECT 'published',now(),now()+interval '1 day',$2,$2,$2
    FROM generate_series(1,201) RETURNING id
   )
   INSERT INTO event_translations(event_id,locale,title,slug,summary,body)
   SELECT id,locale,$1||id,$1||id,'Summary','Event text'
   FROM created CROSS JOIN (VALUES ('en'),('pl')) AS locales(locale)
   RETURNING event_id,slug,locale`, [...args, editor.accountId]);
  const next = t(lang, 'Następna strona', 'Next page');
  const cases = [
   [page, '/admin/news', 'a[href^="/admin/news/"][href*="/edit"]', news.map(row => row.id)],
   [page, '/admin/pages', 'a[href^="/admin/pages/"]', [prefix]],
   [page, '/admin/announcements', 'a[href^="/admin/announcements/"][href$="/edit"]', announcements.map(row => row.id)],
   [page, '/admin/media', 'img[src^="/admin/media/"]', media.map(row => row.id)],
   [page, '/admin/events', 'a[href^="/admin/events/"][href$="/edit"]', events.map(row => row.event_id)],
   [anonymous.page, `/${lang}/news`, `a[href^="/${lang}/news/"]`, news.map(row => row.slug)],
   [anonymous.page, `/${lang}/events`, `a[href^="/${lang}/events/"]`, events.map(row => row.slug)],
  ];
  for (const [subject, path, selector, records] of cases) {
   await visit(subject, path);
   const entries = () => subject.locator(`main ${selector}`).evaluateAll(
    links => links.map(entry => entry.getAttribute('href') || entry.getAttribute('src')),
   );
   const first = await entries();
   await subject.locator('main').getByRole('link', { name: next, exact: true }).click();
   await expect(subject).toHaveURL(/\?page=[12](?:&lang=[a-z]+)?$/);
   await expect.poll(async () => (await entries()).some(target => !first.includes(target)
    && records.some(record => target.includes(record))), { message: path }).toBe(true);
  }
 } finally {
  const pattern = [prefix + '-%'];
  await query("DELETE FROM events WHERE id IN (SELECT event_id FROM event_translations WHERE slug LIKE $1)", pattern);
  await query("DELETE FROM news WHERE slug LIKE $1", pattern);
  await query("DELETE FROM pages WHERE slug LIKE $1", pattern);
  await query("DELETE FROM announcements WHERE title LIKE $1", pattern);
  await query("DELETE FROM media WHERE alt_text LIKE $1", pattern);
 }
});
