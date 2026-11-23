import { expect } from '@playwright/test';
import {
  mailStatus, newAccount, ready, request, SharedRateLimit, startRegistration, status,
  supportRouteStatus, test,
} from './helpers.mjs';

const absent = '00000000-0000-0000-0000-000000000001';

test.beforeEach(async ({ request: api }) => ready(api));

test('anonymous player support navigation and form refusal', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const tickets = `/${lang}/support/tickets`;
  const reports = `/${lang}/support/reports`;

  for (const base of [tickets, reports]) {
    for (const path of [base, `${base}/create`, `${base}/${absent}`, `${base}/invalid-id`]) {
      const response = await status(page.request, 'GET', path, 303);
      expect(response.headers().location).toBe(`/${lang}/login`);
    }
  }

  await page.goto(tickets);
  await expect(page).toHaveURL(new RegExp(`/${lang}/login$`));
  await expect(page.locator('main form[action$="/login"]')).toBeVisible();

  const available = {
    tickets: await supportRouteStatus(page.request, tickets),
    reports: await supportRouteStatus(page.request, reports),
  };
  for (const [method, path] of [
    ['POST', tickets], ['POST', `${tickets}/${absent}/read`],
    ['POST', `${tickets}/${absent}/reply`], ['POST', `${tickets}/${absent}/status`],
    ['PUT', `${tickets}/${absent}/status`], ['POST', reports],
  ]) {
    const section = path === reports ? 'reports' : 'tickets';
    await status(page.request, method, path, available[section] === 'unavailable' ? 404 : 403, { form: {} });
  }
});

test('public origin refuses staff and internal support routes', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const localized = `/${lang}/admin/support/tickets`;
  const staffAreas = ['/admin/support/tickets', '/admin/moderation/reports', '/admin/moderation/enforcement'];
  for (const base of [localized, ...staffAreas]) {
    await status(page.request, 'GET', base, 404);
    await status(page.request, 'GET', `${base}/${absent}`, 404);
    await status(page.request, 'GET', `${base}/invalid-id`, 404);
  }

  for (const [method, path] of [
    ['GET', '/admin/moderation/enforcement/create'],
    ['POST', `${localized}/${absent}/reply`], ['POST', `${localized}/${absent}/status`],
    ['PUT', `${localized}/${absent}/status`], ...staffAreas.map(base => ['POST', `${base}/${absent}`]),
  ]) {
    await status(page.request, method, path, 404, { form: method === 'GET' ? undefined : {} });
  }

  for (const path of [
    'sanctions/stream/read', 'enforcement-commands/stream/read', 'enforcement-commands/commands/read',
    'enforcement-commands/reconcile/read', 'enforcement-commands/results',
  ]) {
    await status(page.request, 'POST', `/internal/v1/support/${path}`, 404, { data: {} });
  }

  for (const path of ['appeals', 'sanctions']) {
    await status(page.request, 'GET', `/${lang}/support/${path}`, 404);
  }
});

test('player registration reaches the verification inbox', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  try {
    await startRegistration(page, lang, newAccount());
    await expect(page.locator('main h1')).toHaveText(lang === 'pl' ? 'Weryfikacja adresu e-mail' : 'E-mail verification');
    await expect(page.getByRole('status')).toHaveText(lang === 'pl'
      ? 'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.'
      : 'Check your inbox for an e-mail with the next steps.');
  } catch (error) {
    test.skip(error instanceof SharedRateLimit, 'Shared registration rate limit after delayed retries');
    const path = info.outputPath('registration.png');
    await page.screenshot({ path }).catch(() => {});
    throw error;
  }
});

test('Mailpit is available for test account verification', async ({ request: api }) => {
  const code = await mailStatus(api);
  test.skip(code === 404 || code === 405, `Staging Mailpit returned HTTP ${code}`);
  expect(code).toBe(200);
  const list = await request(api, 'GET', '/__mailpit/api/v1/messages');
  expect(Array.isArray((await list.json()).messages)).toBe(true);
});
