import { expect } from '@playwright/test';
import {
  fill, formData, mailStatus, openAccountLink, ready, register, restrictToStaging,
  status, submit, supportRouteStatus, test,
} from './helpers.mjs';

const missing = '00000000-0000-0000-0000-000000000001';
const harmlessMarkup = '<review & follow-up>';
const detail = /\/[0-9a-f]{8}-[0-9a-f-]{27}$/;

const copy = {
  en: {
    ticketTitle: 'Support tickets', reportTitle: 'Reports', closed: 'Closed',
    open: 'Open', waiting: 'Waiting for support', submitted: 'Submitted',
    ticketError: 'The request could not be completed. Reload the ticket and try again.',
    reportError: 'Check the report type, matching category, target and evidence lengths.',
  },
  pl: {
    ticketTitle: 'Zgłoszenia pomocy', reportTitle: 'Zgłoszenia', closed: 'Zamknięte',
    open: 'Otwarte', waiting: 'Oczekuje na pomoc', submitted: 'Wysłane',
    ticketError: 'Nie udało się wykonać żądania. Odśwież zgłoszenie i spróbuj ponownie.',
    reportError: 'Sprawdź rodzaj zgłoszenia, pasującą kategorię, cel i długość opisu.',
  },
};

async function post(api, method, data, expected, changes) {
  return status(api, method, data.path, expected, { form: { ...data.values, ...changes } });
}

async function withoutToken(api, method, data) {
  const form = { ...data.values };
  delete form._csrf;
  await status(api, method, data.path, 403, { form });
}

async function asOtherPlayer(browser, info, run) {
  const { locale, viewport, isMobile, hasTouch, proxy, ignoreHTTPSErrors, baseURL } = info.project.use;
  const context = await browser.newContext({
    locale, viewport, isMobile, hasTouch, proxy, ignoreHTTPSErrors, baseURL,
    serviceWorkers: 'block',
  });
  try {
    await restrictToStaging(context);
    const page = await context.newPage();
    await register(page, info.project.metadata.lang);
    await run(page);
  } finally {
    await context.close();
  }
}

test.beforeEach(async ({ request: api }) => ready(api));

async function requirePlayerPrerequisites(api, path) {
  const mounted = await supportRouteStatus(api, path);
  const code = await mailStatus(api);
  test.skip(mounted === 'unavailable' || code === 404 || code === 405,
    `Staging route is ${mounted} and Mailpit returned HTTP ${code}`);
  expect(code).toBe(200);
}

test('ticket creation, follow-up, closing and reopening', async ({ page, browser }, info) => {
  const { lang } = info.project.metadata;
  const words = copy[lang];
  const base = `/${lang}/support/tickets`;
  await requirePlayerPrerequisites(page.request, base);
  const owner = await register(page, lang);
  const subject = `${owner.label} ${harmlessMarkup}`;
  const body = `Account question ${owner.label}`;
  await openAccountLink(page, lang, 'tickets');
  await expect(page.locator('main h1')).toHaveText(words.ticketTitle);
  await status(page.request, 'GET', `${base}?page=invalid`, 400);
  await page.locator(`main a[href="${base}/create"]`).click();
  const form = page.locator(`main form[action="${base}"]`);
  await fill(form, { category: 'account', subject, body });
  const create = await formData(form);
  await withoutToken(page.request, 'POST', create);
  for (const invalid of [
    { subject: ' ' }, { subject: 'x'.repeat(161) }, { request_key: 'invalid-id' }, { category: 'invalid' },
  ]) {
    const response = await post(page.request, 'POST', create, 422, invalid);
    expect(await response.text()).toContain(words.ticketError);
  }

  await form.locator('button').click();
  await expect(page).toHaveURL(detail);
  const ticket = new URL(page.url()).pathname;
  const repeat = await post(page.request, 'POST', create, 302);
  expect(repeat.headers().location).toBe(ticket);
  await page.reload();
  await expect(page.locator('main h2')).toHaveText(subject);
  await expect(page.locator('main article')).toHaveText(new RegExp(body));
  await status(page.request, 'GET', `${ticket}?after=invalid`, 400);
  await status(page.request, 'GET', `${base}/invalid-id`, 400);
  await status(page.request, 'GET', `${base}/${missing}`, 404);

  const reply = page.locator('main form[action$="/reply"]');
  const change = page.locator('main form[action$="/status"]');
  const followUp = `More information ${harmlessMarkup}`;
  await fill(reply, { body: followUp });
  const replyData = await formData(reply);
  const anotherTab = await page.context().newPage();
  await anotherTab.goto(ticket);
  const staleStatus = await formData(anotherTab.locator('main form[action$="/status"]'));
  await withoutToken(page.request, 'POST', replyData);
  for (const invalid of [{ body: '' }, { body: 'x'.repeat(8001) }, { lock_version: 'invalid' }]) {
    await post(page.request, 'POST', replyData, 422, invalid);
  }
  await status(page.request, 'POST', `${base}/invalid-id/reply`, 400, { form: replyData.values });
  await reply.locator('button').click();
  await expect(page.getByText(followUp, { exact: true })).toBeVisible();
  await expect(page.locator('main')).toContainText(words.waiting);
  await post(page.request, 'POST', replyData, 409);
  await post(anotherTab.request, 'POST', staleStatus, 409, { status: 'closed' });
  await anotherTab.close();

  await fill(change, { status: 'closed' });
  const close = await formData(change);
  for (const method of ['POST', 'PUT']) {
    await withoutToken(page.request, method, close);
    for (const invalid of [{ status: '' }, { status: 'x'.repeat(161) }, { status: 'resolved' }, { lock_version: 'invalid' }]) {
      await post(page.request, method, close, 422, invalid);
    }
    await status(page.request, method, `${base}/invalid-id/status`, 400, { form: close.values });
  }
  await change.locator('button').click();
  await page.reload();
  await expect(page.locator('main')).toContainText(words.closed);
  await expect(reply).toHaveCount(0);
  await post(page.request, 'POST', close, 409);

  await fill(change, { status: 'open' });
  const reopen = await formData(change);
  expect((await post(page.request, 'PUT', reopen, 302)).headers().location).toBe(ticket);
  await post(page.request, 'PUT', reopen, 409);
  await page.reload();
  await expect(page.locator('main')).toContainText(words.open);
  await expect(reply).toBeVisible();
  await page.goto(ticket.replace(`/${lang}/`, lang === 'en' ? '/pl/' : '/en/'));
  await expect(page.locator('main h2')).toHaveText(subject);
  await expect(page.locator('main h1')).toHaveText(copy[lang === 'en' ? 'pl' : 'en'].ticketTitle);

  const read = { path: `${ticket}/read`, values: { _csrf: reopen.values._csrf, reply: missing } };
  await withoutToken(page.request, 'POST', read);
  await post(page.request, 'POST', read, 422, { reply: 'invalid-id' });
  expect((await post(page.request, 'POST', read, 302)).headers().location).toBe(ticket);
  expect((await post(page.request, 'POST', read, 302)).headers().location).toBe(ticket);

  await page.goto(base);
  await expect(page.locator(`main a[href="${ticket}"]`)).toHaveCount(1);
  await asOtherPlayer(browser, info, async other => {
    await other.goto(base);
    await expect(other.locator(`main a[href="${ticket}"]`)).toHaveCount(0);
    await status(other.request, 'GET', ticket, 404);
    await other.goto(`${base}/create`);
    const { values: { _csrf } } = await formData(other.locator('main form'));
    await post(other.request, 'POST', read, 404, { _csrf });
    await post(other.request, 'POST', replyData, 404, { _csrf });
    for (const method of ['POST', 'PUT']) await post(other.request, method, close, 404, { _csrf });
  });
});

test('report draft, submission and owner history', async ({ page, browser }, info) => {
  const { lang } = info.project.metadata;
  const words = copy[lang];
  const base = `/${lang}/support/reports`;
  await requirePlayerPrerequisites(page.request, base);
  const owner = await register(page, lang);
  const target = `${owner.label} ${harmlessMarkup}`;
  const evidence = `Example review ${harmlessMarkup}`;
  await openAccountLink(page, lang, 'reports');
  await expect(page.locator('main h1')).toHaveText(words.reportTitle);
  await status(page.request, 'GET', `${base}?page=0`, 422);
  await status(page.request, 'GET', `${base}?page=invalid`, 400);
  await status(page.request, 'GET', `${base}/invalid-id`, 404);
  await page.locator(`main a[href="${base}/create"]`).click();
  const form = page.locator(`main form[action="${base}"]`);
  await fill(form, { report_type: 'player', category: 'harassment', target_reference: target, evidence_summary: evidence });
  const create = await formData(form);

  // Exercise browser validation first. The request still carries the previous values for later replay.
  await fill(form, { target_reference: '' });
  await form.locator('button').click();
  expect(await form.locator('[name=target_reference]').evaluate(el => el.validity.valueMissing)).toBe(true);
  await withoutToken(page.request, 'POST', create);

  const bad = await post(page.request, 'POST', create, 302, { category: 'spam' });
  expect(bad.headers().location).toBe(`${base}/create`);
  await page.reload();
  await expect(page.getByRole('alert')).toContainText(words.reportError);
  await expect(form.locator('[name=target_reference]')).toHaveValue(target);
  await expect(form.locator('[name=evidence_summary]')).toHaveValue(evidence);
  await submit(form, { category: 'harassment' });
  await expect(page).toHaveURL(detail);
  const report = new URL(page.url()).pathname;
  expect((await post(page.request, 'POST', create, 302)).headers().location).toBe(report);
  await page.reload();
  await expect(page.getByText(evidence, { exact: true })).toBeVisible();
  await expect(page.locator('main article')).toContainText(words.submitted);
  await page.locator(`main a[href="${base}"]`).click();
  await expect(page.locator(`main a[href="${report}"]`)).toHaveCount(1);
  await page.goBack();
  await expect(page.getByText(evidence, { exact: true })).toBeVisible();

  await asOtherPlayer(browser, info, async other => {
    await other.goto(base);
    await expect(other.locator(`main a[href="${report}"]`)).toHaveCount(0);
    await status(other.request, 'GET', report, 404);
    await other.goto(`${base}/create`);
    const otherForm = await formData(other.locator('main form'));
    for (const invalid of [
      { target_reference: 'x'.repeat(161) }, { evidence_summary: 'x'.repeat(4001) }, { request_key: 'invalid-id' },
    ]) {
      const response = await post(other.request, 'POST', otherForm, 302, {
        report_type: 'player', category: 'harassment', target_reference: target, ...invalid,
      });
      expect(response.headers().location).toBe(`${base}/create`);
    }
    await other.reload();
    await expect(other.getByRole('alert')).toContainText(words.reportError);
  });
});
