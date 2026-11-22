import { randomUUID } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
import { test as base, expect } from './fixtures.mjs';
export { expect };
import { csrfRejected, fixtureDatabaseURL, login, password } from './helpers.mjs';
import { enrollMfa, loginWithRecoveryCode, query } from './staff-rbac-helpers.mjs';

export const locales = ['pl', 'en'];
export const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
export const key = prefix => `${prefix}-${randomUUID()}`;
export const stamp = minutes => new Date(Date.now() + minutes * 60_000).toISOString();
export const localized = (lang, pl, en) => lang === 'pl' ? pl : en;

export function binary(args, env = {}) {
 const executable = process.env.PORTAL_E2E_BINARY ||
   resolve(process.env.CARGO_TARGET_DIR || '../target', 'debug/oteryn-portal');
 const databaseURL = fixtureDatabaseURL();
 try {
  return { status: 0, stdout: execFileSync(executable, args, {
   env: { ...process.env, ...env, OTERYN_DATABASE_URL: databaseURL }, stdio: ['ignore', 'pipe', 'pipe'], encoding: 'utf8',
  }) };
 } catch (error) {
  if (typeof error.status !== 'number') throw new Error('Could not run the local portal binary');
  const refusal = String(error.stderr).match(/^Publication refused: ([^\r\n]+)$/m)?.[1];
  return { status: error.status, refusal };
 }
}

async function actor(browser, project, role) {
 const lang = project.metadata.locale;
 const context = await browser.newContext({
  baseURL: origin, ignoreHTTPSErrors: true, locale: lang, viewport: project.use.viewport,
 });
 try {
  const page = await context.newPage();
  if (role) {
   const account = { email: `${key('content')}@example.invalid`, password: password() };
   const seeded = binary(['seed', 'account', JSON.stringify({ ...account, locale: lang })]);
   expect(seeded.status, 'seed verified test actor').toBe(0);
   const { accountId } = JSON.parse(seeded.stdout);
   if (role !== 'player') {
    await query('INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2',
     [accountId, role]);
   }
   await login(page, lang, account);
   await enrollMfa(page, lang, account);
   const recovery = page.locator('textarea[readonly]').first();
   await expect(recovery).toBeVisible();
   const code = await recovery.inputValue();
   // Enrollment ends the original session. Start the MFA challenge in a fresh
   // cookie jar so the enrollment and sign-in CSRF sessions cannot overlap.
   await context.clearCookies();
   await loginWithRecoveryCode(page, lang, account, code);
   await visit(page, `/${lang}/`);
   const csrf = await page.locator('form[action$="/logout"] [name="_csrf"]').inputValue();
   return { context, page, accountId, csrf };
  }
  await visit(page, `/${lang}/login`);
  const csrf = await page.locator('main form[action$="/login"] [name="_csrf"]').inputValue();
  return { context, page, csrf };
 } catch (error) {
  await context.close();
  throw error;
 }
}

export async function visit(page, path, status = 200) {
 const response = await page.goto(path);
 expect(response.status(), path).toBe(status);
 return response;
}

export async function request(page, path, method, status, options = {}) {
 const response = await page.request.fetch(path, {
  ...options, method, maxRedirects: 0, headers: { origin, ...options.headers },
 });
 expect(response.status(), `${method} ${path}`).toBe(status);
 return response;
}

export async function formData(form) {
 return form.evaluate(element => Object.fromEntries(new FormData(element)));
}

export async function send(form, changes, status, method = 'POST', target) {
 const page = form.page();
 const path = target || (await form.getAttribute('action')) || page.url();
 return request(page, path, method, status, { form: { ...await formData(form), ...changes } });
}

export async function submitForm(form, values = {}, button = 'button') {
 const page = form.page();
 const path = (await form.getAttribute('action')) || page.url();
 await csrfRejected(page, path, { ...await formData(form), ...values });
 for (const [name, value] of Object.entries(values)) {
  const field = form.locator(`[name="${name}"]`);
  if (await field.evaluate(element => element.tagName === 'SELECT')) await field.selectOption(value);
  else if (typeof value === 'boolean') await field.setChecked(value);
  else await field.fill(value);
 }
 await form.locator(button).click();
}

export async function link(page, href, label) {
 const item = page.locator(`main a[href="${href}"]`);
 if (label) await expect(item).toHaveText(label);
 await item.click();
}

export async function title(page, expected, lang) {
 await expect(page.locator('html')).toHaveAttribute('lang', lang);
 await expect(page.locator('main h1')).toHaveText(expected);
}

export const editPath = page => new URL(page.url()).pathname;
export const formFor = (page, suffix) => page.locator(`main form[action$="${suffix}"]`);
export const mainForm = page => page.locator('main form').first();
export async function labels(form, lang, entries) {
 for (const [name, pl, en] of entries) {
  await expect(form.getByLabel(localized(lang, pl, en), { exact: true })).toHaveAttribute('name', name);
 }
}

export async function refuse({anonymous, player, support}, path, method = 'GET', form = {}, multipart) {
 await Promise.all([[anonymous, 303], [player, 403], [support, 403]].map(async ([subject, status]) => {
  const csrf = subject.csrf;
  const payload = multipart ? { multipart: { _csrf: csrf, ...multipart } } :
   { form: { ...form, _csrf: csrf } };
  const response = await request(subject.page, path, method, status,
   method === 'GET' ? {} : payload);
  if (status === 303) expect(response.headers().location).toBe('/login');
 }));
}
export const png = {
 name: 'content.png', mimeType: 'image/png',
 buffer: Buffer.from('iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAHklEQVR4nGNQzrP5T03MMGrgqIGjBo4aOGrgSDUQANpkzt/P0xIDAAAAAElFTkSuQmCC', 'base64'),
};


export const test = base.extend({
 content: [async ({browser}, use, {project}) => {
  const roles = {
   anonymous: null, editor: 'content_editor', admin: 'platform_admin',
   player: 'player', support: 'support',
  };
  const subjects = {};
  try {
   for (const [name, role] of Object.entries(roles)) {
    subjects[name] = await actor(browser, project, role);
   }
   await use(subjects);
  } finally {
   await Promise.all(Object.values(subjects).map(({context}) => context.close()));
  }
 }, {scope: 'worker', timeout: 180_000}],
});
