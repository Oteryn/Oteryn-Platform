import { randomUUID } from 'node:crypto';
import { mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { setTimeout as delay } from 'node:timers/promises';
import { expect } from '@playwright/test';
import { inboxPath, mailLink, password, safeForm, send, status, test, visit } from './helpers.mjs';

const id = '00000000-0000-4000-8000-000000000000';
const wiki = `/admin/wiki/articles/${id}`;
const admin = {
  news: [
    ['GET', '/admin/news'], ['POST', '/admin/news'], ['GET', '/admin/news/create'],
    ['GET', `/admin/news/${id}/edit`], ['POST', `/admin/news/${id}`], ['PUT', `/admin/news/${id}`],
    ['DELETE', `/admin/news/${id}`], ['POST', `/admin/news/${id}/delete`],
  ],
  pages: [
    ['GET', '/admin/pages'], ['GET', '/admin/pages/sim-m-content-sample/en'],
    ['POST', '/admin/pages/sim-m-content-sample/en'],
    ...['terms-of-service', 'privacy-policy', 'cookie-policy'].flatMap(slug =>
      ['GET', 'POST'].map(method => [method, `/admin/pages/${slug}/en`])),
  ],
  events: [
    ['GET', '/admin/events'], ['POST', '/admin/events'], ['GET', '/admin/events/create'],
    ['GET', `/admin/events/${id}/edit`],
    ...['PUT', 'POST'].flatMap(method => [[method, `/admin/events/${id}`], [method, `/admin/events/${id}/status`]]),
  ],
  announcements: [
    ['GET', '/admin/announcements'], ['POST', '/admin/announcements'], ['GET', '/admin/announcements/create'],
    ['GET', `/admin/announcements/${id}/edit`], ['POST', `/admin/announcements/${id}/edit`],
  ],
  media: [
    ['GET', '/admin/media'], ['POST', '/admin/media'], ['GET', `/admin/media/${id}/content`],
    ['GET', `/admin/media/${id}/thumbnail`], ['DELETE', `/admin/media/${id}`],
    ['POST', `/admin/media/${id}/delete`],
  ],
  wiki: [
    ['GET', '/admin/wiki'], ['GET', '/admin/wiki/articles'], ['POST', '/admin/wiki/articles'],
    ['GET', '/admin/wiki/articles/create'], ['GET', wiki + '/edit'], ['POST', wiki],
    ['GET', wiki + '/preview/pl'], ['GET', wiki + '/preview/en'], ['GET', wiki + '/revisions'],
    ['POST', wiki + '/revisions/1/restore'],
    ...['submit-review', 'return-draft', 'publish', 'unpublish', 'archive'].map(action => ['POST', `${wiki}/${action}`]),
  ],
};

test('CMS routes are unavailable on the staging host', async ({ request }) => {
  await status(request, 'GET', '/admin', 404);
  for (const routes of Object.values(admin)) {
    for (const [method, path] of routes) {
      await status(request, method, path, 404, method === 'GET' ? {} : {
        headers: { origin: 'https://staging.oteryn.com' },
        ...(path === '/admin/media' ? { multipart: { alt_text: '' } } : { form: {} }),
      });
    }
  }
});

test('Player registration follows the email verification state', async ({ page, request }, info) => {
  const lang = info.project.metadata.locale;
  const pl = lang === 'pl';
  const email = `sim-m-content-${randomUUID()}-1@example.test`;
  const chosenPassword = password();
  const form = page.locator(`main form[action="/${lang}/register"]`);
  const start = Date.now() - 2000;

  await visit(page, `/${lang}/`, 200);
  await page.locator(`a[href="/${lang}/register"]:visible`).first().click();
  await expect(page).toHaveURL(url => url.pathname === `/${lang}/register`);
  const badEmail = form.locator('input[name=email]');
  await badEmail.fill('not-an-email');
  expect(await badEmail.evaluate(input => input.validity.typeMismatch)).toBe(true);

  let challenge;
  page.on('response', response => {
    if (new URL(response.url()).pathname === '/register/abuse/challenge') challenge = response;
  });

  const attempts = 4;
  let submitted;
  let limited;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    challenge = undefined;
    try {
      [submitted] = await Promise.all([
        page.waitForResponse(
          r => r.request().method() === 'POST' && new URL(r.url()).pathname === `/${lang}/register`,
          { timeout: 45_000 },
        ),
        safeForm(form, {
          email, password: chosenPassword, password_confirmation: chosenPassword, terms: true, privacy: true,
        }),
      ]);
    } catch {
      throw new Error('Could not submit the player registration form.');
    }
    limited = submitted.status() !== 303
      && [submitted, challenge].find(response => response?.status() === 429);
    if (!limited || attempt === attempts - 1) break;
    const retryAfter = Number((await limited.allHeaders())['retry-after']) || 60;
    const viewportDelay = info.project.use.viewport.width === 390 ? 4 : 1;
    await delay(Math.min(65, Math.max(10, retryAfter + viewportDelay)) * 1000);
    await visit(page, `/${lang}/register`, 200);
  }
  test.skip(Boolean(limited), 'Shared staging signup budget was exhausted after four attempts.');
  if (submitted.status() === 503) {
    await expect.soft(page.locator('main')).toContainText(pl
      ? 'Rejestracja jest obecnie niedostępna. Spróbuj ponownie później.'
      : 'Registration is currently unavailable. Please try again later.');
    const folder = new URL('./artifacts/', import.meta.url);
    await mkdir(folder, { recursive: true });
    const screenshot = new URL(`registration-${info.project.name}.png`, folder);
    try {
      await page.screenshot({ path: fileURLToPath(screenshot), timeout: 15_000 });
    } catch {
      // The unavailable page has no form or other personal fields.
      const cdp = await page.context().newCDPSession(page);
      try {
        const { data } = await cdp.send('Page.captureScreenshot', { format: 'png' });
        await writeFile(screenshot, Buffer.from(data, 'base64'));
      } finally {
        await cdp.detach();
      }
    }
  }
  const challengeStatus = challenge?.status() ?? 'no response';
  expect(submitted.status(), `registration redirects to inbox confirmation (challenge ${challengeStatus})`).toBe(303);
  await expect(page.locator('main [role=status]')).toHaveText(pl
    ? 'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.'
    : 'Check your inbox for an e-mail with the next steps.');
  await expect(page).toHaveURL(url => url.pathname === `/${lang}/register/check-inbox`);
  await page.reload({ waitUntil: 'domcontentloaded' });
  await expect(page).toHaveURL(url => url.pathname === `/${lang}/register/check-inbox`);

  const mail = await send(request, 'GET', inboxPath);
  const mailStatus = mail.status();
  await mail.dispose();
  const available = mailStatus === 200;
  expect([200, 404, 405], 'Mailpit availability').toContain(mailStatus);
  if (available) {
    const link = await mailLink(request, email, start, /verif|potwierd/i, /^\/(?:en\/|pl\/)?verify-email$/);
    expect(Boolean(link), 'verification mail delivered within 60 seconds').toBe(true);
    try {
      await page.goto(link, { waitUntil: 'domcontentloaded' });
      await page.locator('main form button').click();
    } catch {
      throw new Error('Could not complete email verification.');
    }
    await expect(page.locator('main [role=status]')).toHaveText(pl
      ? 'Twój adres e-mail został zweryfikowany.'
      : 'Your e-mail has been verified.');
  } else {
    info.annotations.push({ type: 'staging limitation', description: `Mailpit returned ${mailStatus}; verification mail could not be read.` });
  }

  await visit(page, `/${lang}/login`, 200);
  try {
    await safeForm(page.locator(`main form[action="/${lang}/login"]`), { email, password: chosenPassword });
  } catch {
    throw new Error('Could not submit the player sign-in form.');
  }
  if (available) {
    await expect(page).toHaveURL(url => url.pathname === `/${lang}/`);
  } else {
    await expect(page).toHaveURL(url => url.pathname === `/${lang}/login`);
    await expect(page.locator('main [role=alert]')).toHaveText(pl
      ? 'Podane dane logowania są nieprawidłowe.'
      : 'The provided credentials are invalid.');
  }
  await visit(page, '/admin/news', 404);
});
