import {randomUUID} from 'node:crypto';
import {test, expect} from '../fixtures.mjs';
import {csrfRejected, submit} from '../helpers.mjs';
import {assertShell} from '../shell.mjs';
import {query, seededStaff} from '../staff-rbac-helpers.mjs';

const schedule = 'button[value="schedule"]';

async function scheduleForm(page, selector, fields, status) {
 const form = page.locator(selector);
 const values = await form.evaluate(el => Object.fromEntries(new FormData(el)));
 const target = new URL(await form.getAttribute('action') || page.url(), page.url());
 const payload = {...values, ...fields, action: 'schedule'};
 if (status !== undefined) {
   const response = await page.request.post(target.href, {
     form: payload, headers: {origin: target.origin}, maxRedirects: 0,
   });
   expect(response.status()).toBe(status);
   return;
 }
 await csrfRejected(page, target.href, payload);
 for (const [name, value] of Object.entries(fields)) {
   await form.locator(`[name="${name}"]`).fill(value);
 }
 await form.locator(schedule).click();
}

async function expectPublic(page, key, status) {
 const responses = await Promise.all(['en', 'pl'].flatMap(lang =>
   ['news', 'pages'].map(async section => {
     const response = await page.request.get(`/${lang}/${section}/${key}`);
     return {response, title: `${lang.toUpperCase()} ${section} ${key}`};
   })));
 for (const {response, title} of responses) {
   expect(response.status()).toBe(status);
   if (status === 200) expect(await response.text()).toContain(title);
 }
}

test('staff schedules bilingual news and pages through publication and expiry', async ({page, browser}, info) => {
 const {locale: lang} = info.project.metadata;
 const key = `launch-${randomUUID()}`;
 await seededStaff(page, lang, 'platform_admin');

 const readRole = `reader_${randomUUID().replaceAll('-', '')}`;
 await query(
   "WITH role AS (INSERT INTO roles(key) VALUES($1) RETURNING id) INSERT INTO role_permissions(role_id,permission) SELECT id,'admin.access' FROM role",
   [readRole],
 );
 const reader = await browser.newContext({locale: lang, ignoreHTTPSErrors: true, baseURL: new URL(page.url()).origin});
 try {
   const readerPage = await reader.newPage();
   await seededStaff(readerPage, lang, readRole);
   await readerPage.goto(`/${lang}/account/sudo`);
   const token = await readerPage.locator('form[method="post"] input[name="_csrf"]').first().inputValue();
   for (const path of ['/admin/news', `/admin/pages/${key}/en`]) {
     const denied = await readerPage.request.post(path, {
       form: {
         _csrf: token, action: 'schedule', slug: key, title: 'Launch', body: 'Launch details',
         title_pl: 'Publikacja', body_pl: 'Szczegóły publikacji',
         schedule_at: '2030-05-10T12:00:00Z', schedule_until: '2030-05-10T13:00:00Z',
       },
       headers: {origin: new URL(readerPage.url()).origin},
       maxRedirects: 0,
     });
     expect(denied.status()).toBe(403);
   }
 } finally {
   await reader.close();
 }

 for (const locale of ['en', 'pl']) {
   expect((await page.goto(`/admin/pages/${key}/${locale}`)).status()).toBe(200);
   await submit(page, 'main form:has([name="title"])', {title: `${locale.toUpperCase()} pages ${key}`, body: 'Launch details'});
   await expect(page).toHaveURL(/\/admin\/pages(?:#main)?$/);
 }
 await page.goto(`/admin/news?lang=${lang}`);
 await page.locator('main a[href^="/admin/news/create"]').click();
 const newsForm = 'main form:has([name="slug"])';
 const pageForm = 'main form:has([name="schedule_at"])';
 const newsFields = {
   slug: key, title: `EN news ${key}`, body: 'Launch announcement',
   title_pl: `PL news ${key}`, body_pl: 'Ogłoszenie',
 };
 await expect(page.locator(newsForm).locator(schedule)).toHaveText(lang === 'pl' ? 'Zaplanuj w obu językach' : 'Schedule both languages');
 await assertShell(page);
 for (const [schedule_at, schedule_until] of [
   ['2030-05-10T12:00:00+25:00', ''],
   ['2030-05-10T12:00:00+02:00', '2030-05-10T09:00:00Z'],
 ]) {
   await scheduleForm(page, newsForm, {...newsFields, schedule_at, schedule_until}, 422);
 }
 const starts = Date.now() + 22_000;
 const ends = starts + 15_000;
 const window = {
   schedule_at: new Date(starts + 2 * 3_600_000).toISOString().replace('Z', '+02:00'),
   schedule_until: new Date(ends).toISOString(),
 };
 await scheduleForm(page, newsForm, {...newsFields, ...window});
 await expect(page).toHaveURL(/\/admin\/news\/[a-f0-9-]+\/edit/);
 const newsEdit = new URL(page.url()).pathname;
 await page.goto(`/admin/pages/${key}/en`);
 await expect(page.locator(pageForm).locator(schedule)).toHaveText(lang === 'pl' ? 'Zaplanuj w obu językach' : 'Schedule both languages');
 await assertShell(page);
 await scheduleForm(page, pageForm, window);
 await expect(page).toHaveURL(/\/admin\/pages(?:#main)?$/);
 await expectPublic(page, key, 404);

 const queued = lang === 'pl' ? 'Zaplanowano' : 'Scheduled';
 const expired = lang === 'pl' ? 'Wygasło' : 'Expired';
 await expect(page.locator('main li').filter({hasText: key}).filter({hasText: queued})).toHaveCount(2);
 await page.goto(`/admin/news?lang=${lang}`);
 const newsRow = page.locator('main li').filter({has: page.locator(`a[href^="${newsEdit}"]`)});
 await expect(newsRow).toContainText(queued);
 await assertShell(page);

 await expect.poll(async () => (await page.request.get(`/en/pages/${key}`)).status(), {timeout: 30_000}).toBe(200);
 await expectPublic(page, key, 200);
 await page.reload();
 await expect(newsRow).toContainText(lang === 'pl' ? 'Opublikowano' : 'Published');
 const publicList = await page.request.get(`/${lang}/news`);
 expect(await publicList.text()).toContain(`/${lang}/news/${key}`);
 await expect.poll(async () => (await page.request.get(`/en/pages/${key}`)).status(), {timeout: 30_000}).toBe(404);
 await expectPublic(page, key, 404);
 const expiredList = await page.request.get(`/${lang}/news`);
 expect(await expiredList.text()).not.toContain(`/${lang}/news/${key}`);
 await page.reload();
 await expect(newsRow).toContainText(expired);
 await page.goto('/admin/pages');
 await expect(page.locator('main li').filter({hasText: key}).filter({hasText: expired})).toHaveCount(2);
});
