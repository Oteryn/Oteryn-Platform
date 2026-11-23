import { test, expect } from '@playwright/test';
import {
  captureForm, click, formFor, MailUnavailable, onceAfterRedeploy, pathOf, playerId, SharedBudget,
  post, ready, register, runId, secondContext, attachAccessRouting, visit,
} from './helpers.mjs';

const copy = {
  en: {
    login: 'Sign in', tickets: 'Support tickets', categories: ['Account', 'Character', 'Technical', 'Payment', 'Other'],
    open: 'Open', closed: 'Closed', waiting: 'Waiting for support',
    ticketError: 'The request could not be completed. Reload the ticket and try again.',
    submitted: 'Submitted', invalidReport: 'Check the report type, matching category, target and evidence lengths. Reload the form if the request key is invalid.',
    rateLimited: 'Too many reports. Try again in one minute.',
  },
  pl: {
    login: 'Zaloguj się', tickets: 'Zgłoszenia pomocy', categories: ['Konto', 'Postać', 'Techniczne', 'Płatność', 'Inne'],
    open: 'Otwarte', closed: 'Zamknięte', waiting: 'Oczekuje na pomoc',
    ticketError: 'Nie udało się wykonać żądania. Odśwież zgłoszenie i spróbuj ponownie.',
    submitted: 'Przesłane', invalidReport: 'Sprawdź rodzaj zgłoszenia, pasującą kategorię, cel i długość opisu. Jeśli klucz żądania jest nieprawidłowy, odśwież formularz.',
    rateLimited: 'Zbyt wiele zgłoszeń. Spróbuj ponownie za minutę.',
  },
};

const categories = ['account', 'character', 'technical', 'payment', 'other'];
const ticketStatus = (page, status) => expect(page.locator('main > p').first()).toContainText(`Status: ${status}`);

async function createTickets(page, language, prefix) {
  const base = `/${language}/support/tickets`;
  const text = copy[language];
  const tickets = [];
  for (const category of categories) {
    await visit(page, `${base}/create`);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(text.tickets);
    const form = formFor(page, '/support/tickets');
    await expect(form.locator('[name=category] option')).toHaveText(text.categories);
    await expect(form.locator('input[type=file]')).toHaveCount(0);
    const subject = `${prefix} ${category}`;
    const body = category === 'technical'
      ? `<script type="text/plain">${prefix} sample text</script>`
      : `${prefix} question about ${category}`;
    const fields = { category, subject, body };
    const submission = await captureForm(form);
    if (category === 'account') {
      await form.getByRole('button').click();
      await expect(form.locator('[name=subject]:invalid')).toBeVisible();
      await expect(page).toHaveURL(`${base}/create`);
    }
    await click(form, fields);
    await expect(page).toHaveURL(new RegExp(`${base}/[a-f0-9-]{36}$`));
    const path = pathOf(page);
    await expect(page.getByRole('heading', { name: subject, exact: true })).toBeVisible();
    await expect(page.locator('main article p')).toHaveText(body);
    await ticketStatus(page, text.open);
    if (category === 'account') {
      const duplicate = await post(page, submission, fields);
      expect(duplicate.status(), 'repeated submission returns the original ticket').toBe(302);
      expect(duplicate.headers().location).toBe(path);
    }
    if (category === 'technical') await expect(page.locator('main article script')).toHaveCount(0);
    tickets.push({ path, subject });
  }
  await visit(page, base);
  for (const { path, subject } of tickets) {
    await expect(page.getByRole('link', { name: subject, exact: true })).toHaveAttribute('href', path);
  }
  return tickets[0];
}

async function changeTicket(page, language, ticket, run) {
  const text = copy[language];
  const secondTab = await page.context().newPage();
  await visit(page, ticket.path);
  await visit(secondTab, ticket.path);
  const reply = formFor(page, '/reply');
  const originalReply = await captureForm(reply);
  const tooLong = await post(page, originalReply, { body: 'a'.repeat(8001) });
  expect(tooLong.status(), 'reply beyond the character limit').toBe(422);
  expect(await tooLong.text()).toBe(text.ticketError);

  const message = `${run} follow-up`;
  await click(reply, { body: message });
  await ticketStatus(page, text.waiting);
  await expect(page.getByText(message, { exact: true })).toBeVisible();
  const staleStatus = await captureForm(formFor(secondTab, '/status'));
  const stale = await post(secondTab, staleStatus, { status: 'closed' });
  expect(stale.status(), 'closing in a stale tab').toBe(409);
  expect(await stale.text()).toBe(text.ticketError);
  await secondTab.reload();
  await ticketStatus(secondTab, text.waiting);
  await click(formFor(secondTab, '/status'), { status: 'closed' });
  await ticketStatus(secondTab, text.closed);
  await expect(formFor(secondTab, '/reply')).toHaveCount(0);

  const otherLanguage = language === 'pl' ? 'en' : 'pl';
  await visit(page, ticket.path.replace(`/${language}/`, `/${otherLanguage}/`));
  await expect(page.locator('html')).toHaveAttribute('lang', otherLanguage);
  await ticketStatus(page, copy[otherLanguage].closed);
  await click(formFor(page, '/status'), { status: 'open' });
  await ticketStatus(page, copy[otherLanguage].open);
  await page.goBack({ waitUntil: 'domcontentloaded' });
  await visit(page, ticket.path);
  await ticketStatus(page, text.open);
  await expect(page.getByText(message, { exact: true })).toHaveCount(1);
  await secondTab.close();
}

async function createReports(page, language, target, ticket, run) {
  const base = `/${language}/support/reports`;
  const text = copy[language];
  const inputs = [
    { report_type: 'player', category: 'conduct', target_reference: target, evidence_summary: `Simulation ${run} player report` },
    { report_type: 'content', category: 'spam', target_reference: new URL(ticket.path, page.url()).href, evidence_summary: `Simulation ${run} content report` },
  ];
  const reports = [];
  for (const fields of inputs) {
    await visit(page, `${base}/create`);
    const form = formFor(page, '/support/reports');
    await click(form, fields);
    await expect(page).toHaveURL(new RegExp(`${base}/[a-f0-9-]{36}$`));
    reports.push(pathOf(page));
    await expect(page.locator('main article')).toContainText(text.submitted);
    await expect(page.locator('main article')).toContainText(fields.evidence_summary);
  }
  await visit(page, `${base}/create`);
  const form = formFor(page, '/support/reports');
  await form.getByRole('button').click();
  await expect(form.locator('[name=target_reference]:invalid')).toBeVisible();
  const submission = await captureForm(form);
  const invalid = await post(page, submission, {
    ...inputs[1],
    evidence_summary: 'a'.repeat(4001),
  });
  expect(invalid.status(), 'oversized report reopens the form with guidance').toBe(302);
  expect(invalid.headers().location).toBe(`${base}/create`);
  await visit(page, `${base}/create`);
  await expect(page.getByRole('alert')).toHaveText(text.invalidReport);
  const limit = await post(page, await captureForm(form), inputs[1]);
  expect(limit.status(), 'fourth report attempt in the same window').toBe(429);
  expect(limit.headers()['retry-after']).toBe('60');
  expect(await limit.text()).toBe(text.rateLimited);
  await visit(page, base);
  for (const path of reports) await expect(page.locator(`main article a[href="${path}"]`)).toBeVisible();
  await expect(page.locator('main article')).toHaveCount(2);
  return reports;
}

test.afterEach(async ({ context }) => {
  await context.unrouteAll({ behavior: 'wait' });
});

test('player opens tickets and files reports', async ({ page, browser, request }, info) => {
  const language = info.project.metadata.language;
  await attachAccessRouting(page.context());
  await onceAfterRedeploy(info, page, async () => {
    await ready(request);
    const run = runId();
    const otherContext = await secondContext(browser, info);
    try {
      await test.step('Open support entry points before signing in', async () => {
        for (const form of ['tickets', 'reports']) {
          await visit(page, `/${language}/support/${form}/create`);
          await expect(page).toHaveURL(`/${language}/login`);
          await expect(page.getByRole('heading', { level: 1 })).toHaveText(copy[language].login);
        }
      });
      await test.step('Create and verify a fresh player', async () => {
        await register(page, language, run, 1, info);
      });
      const ticket = await test.step('Submit and reload each ticket category', () => createTickets(page, language, run));
      await test.step('Reply and move between ticket states in two tabs', () => changeTicket(page, language, ticket, run));
      const other = await otherContext.newPage();
      const target = await register(other, language, run, 2, info);
      const reports = await test.step('File two reports and check submission feedback', async () => {
        return createReports(page, language, await playerId(other, language, target), ticket, run);
      });
      await test.step('Open the reporters’ URLs as another signed-in player', async () => {
        await visit(other, ticket.path, 404);
        await expect(other.locator('body')).toHaveText(copy[language].ticketError);
        for (const report of reports) {
          await visit(other, report, 404);
          await expect(other.locator('main article')).toHaveCount(0);
        }
      });
    } catch (error) {
      if (!(error instanceof MailUnavailable || error instanceof SharedBudget)) throw error;
      test.skip(true, error.message);
    } finally {
      await otherContext.unrouteAll({ behavior: 'wait' });
      await otherContext.close();
    }
  });
});
