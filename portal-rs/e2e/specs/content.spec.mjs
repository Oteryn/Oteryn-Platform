import {test,expect} from '../fixtures.mjs';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {submit} from '../helpers.mjs';
import {assertShell} from '../shell.mjs';

test.beforeEach(async({context},info)=>{
 const state=JSON.parse(fs.readFileSync(path.join(process.env.PORTAL_E2E_AUTH_DIRECTORY,`staff-${info.project.name}.json`),'utf8'));
 await context.addCookies(state.cookies);
});
const slug=prefix=>`${prefix}-${crypto.randomUUID()}`;

test('editorial news: create bilingual publication, edit, read and delete',async({page},info)=>{
 const key=slug('browser-news');
 expect((await page.goto('/admin/news/create')).status()).toBe(200);
 await assertShell(page);
 const date=new Date(Date.now()-60_000).toISOString();
 await submit(page,'main form',{slug:key,title:'English browser news',body:'## Editorial content\n\nPublished through the staff form.',published_at:date,title_pl:'Polskie wiadomości przeglądarkowe',body_pl:'## Treść redakcyjna\n\nPublikacja przez formularz zespołu.',translation_published_at:date});
 await expect(page).toHaveURL(/\/admin\/news\/[a-f0-9-]+\/edit$/);
 const edit=new URL(page.url()).pathname;
 await assertShell(page);
 await submit(page,'main form:not([action$="/delete"])',{body:'## Editorial content\n\nEdited through the staff form.',body_pl:'## Treść redakcyjna\n\nAktualizacja przez formularz zespołu.'});
 for(const lang of [info.project.metadata.locale,info.project.metadata.locale==='pl'?'en':'pl']) {
   expect((await page.goto(`/${lang}/news/${key}`)).status()).toBe(200);
   await expect(page.locator('main h1')).toHaveText(lang==='pl'?'Polskie wiadomości przeglądarkowe':'English browser news');
   await expect(page.locator('main')).toContainText(lang==='pl'?'Aktualizacja przez formularz zespołu.':'Edited through the staff form.');
   await assertShell(page);
 }
 await page.goto(edit);await submit(page,'main form[action$="/delete"]');
 expect((await page.goto(`/${info.project.metadata.locale}/news/${key}`)).status()).toBe(404);
 await assertShell(page,{status:404});
});

test('editorial event: create, publish and read both translations',async({page},info)=>{
 const key=slug('browser-event');
 expect((await page.goto('/admin/events/create')).status()).toBe(200);
 await assertShell(page);
 await submit(page,'main form',{starts_at:new Date(Date.now()-3_600_000).toISOString().slice(0,16),ends_at:new Date(Date.now()+86_400_000).toISOString().slice(0,16),en_title:'English browser event',en_slug:key,en_summary:'Public editorial event.',en_body:'## Event details\n\nPublished by the staff browser.',pl_title:'Polskie wydarzenie przeglądarkowe',pl_slug:key,pl_summary:'Publiczne wydarzenie redakcyjne.',pl_body:'## Szczegóły wydarzenia\n\nPublikacja przez przeglądarkę zespołu.'});
 await expect(page).toHaveURL(/\/admin\/events\/[a-f0-9-]+\/edit$/);
 await assertShell(page);
 await page.locator('main form[action$="/status"] select[name=status]').selectOption('published');
 await submit(page,'main form[action$="/status"]');
 for(const lang of [info.project.metadata.locale,info.project.metadata.locale==='pl'?'en':'pl']) {
   expect((await page.goto(`/${lang}/events/${key}`)).status()).toBe(200);
   await expect(page.locator('main h1')).toHaveText(lang==='pl'?'Polskie wydarzenie przeglądarkowe':'English browser event');
   await assertShell(page);
 }
});

test('wiki editorial article: draft, preview, review, publish, history and unpublish',async({page},info)=>{
 const key=slug('browser-guide');
 expect((await page.goto('/admin/wiki/articles/create')).status()).toBe(200);
 await assertShell(page);
 await submit(page,'main form',{en_title:'English browser guide',en_slug:key,en_summary:'An editorial guide.',en_markdown:'## Guide details\n\nWritten through the staff form.',pl_title:'Polski poradnik przeglądarkowy',pl_slug:key,pl_summary:'Poradnik redakcyjny.',pl_markdown:'## Szczegóły poradnika\n\nTreść z formularza zespołu.',note:'Local browser editorial qualification.'});
 await expect(page).toHaveURL(/\/admin\/wiki\/articles\/[a-f0-9-]+\/edit$/);
 const edit=new URL(page.url()).pathname;
 const base=edit.slice(0,-'/edit'.length);
 await assertShell(page);
 expect((await page.request.get(`/${info.project.metadata.locale}/wiki/articles/${key}`)).status()).toBe(404);
 for(const lang of ['pl','en']) {
   expect((await page.goto(`${base}/preview/${lang}`)).status()).toBe(200);
   await assertShell(page);
 }
 await page.goto(edit);await submit(page,'main form[action$="/submit-review"]');
 await submit(page,'main form[action$="/publish"]');
 for(const lang of [info.project.metadata.locale,info.project.metadata.locale==='pl'?'en':'pl']) {
   expect((await page.goto(`/${lang}/wiki/articles/${key}`)).status()).toBe(200);
   await expect(page.locator('main h1')).toHaveText(lang==='pl'?'Polski poradnik przeglądarkowy':'English browser guide');
   await assertShell(page);
 }
 expect((await page.goto(`${base}/revisions`)).status()).toBe(200);await assertShell(page);
 await page.goto(edit);await submit(page,'main form[action$="/unpublish"]');
 expect((await page.request.get(`/${info.project.metadata.locale}/wiki/articles/${key}`)).status()).toBe(404);
});

test('media: upload, retrieve normalized content and thumbnail, then delete',async({page})=>{
 const alt=`Local browser image ${crypto.randomUUID()}`;
 expect((await page.goto('/admin/media')).status()).toBe(200);
 await assertShell(page);
 await page.locator('main form[action="/admin/media"] input[type=file]').setInputFiles({name:'local.png',mimeType:'image/png',buffer:Buffer.from('iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAHklEQVR4nGNQzrP5T03MMGrgqIGjBo4aOGrgSDUQANpkzt/P0xIDAAAAAElFTkSuQmCC','base64')});
 await submit(page,'main form[action="/admin/media"]',{alt_text:alt});
 const image=page.getByRole('img',{name:alt,exact:true});await expect(image).toBeVisible();
 const thumbnail=await image.getAttribute('src');
 const content=thumbnail.replace(/\/thumbnail$/,'/content');
 for(const target of [thumbnail,content]) {
   const response=await page.request.get(target);expect(response.status()).toBe(200);
   expect(response.headers()['content-type']).toBe('image/png');
   expect((await response.body()).length).toBeGreaterThan(0);
 }
 await assertShell(page);
 await submit(page,`main form[action="${thumbnail.replace(/\/thumbnail$/,'/delete')}"]`);
 await expect(page.getByRole('img',{name:alt,exact:true})).toHaveCount(0);
 expect((await page.request.get(content)).status()).toBe(404);
});
