import { randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { login, submit } from '../helpers.mjs';
import { cookie, expectSession, otherBrowser } from '../security/auth-fixtures.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { expectCsrfRefusal, expectRedirect, reloadGet, replay, signIn, snapshot, submitForm } from './forms.mjs';

const copy = {
  en: {
    open: 'Open', waiting: 'Waiting for you', title: 'Support tickets',
    conflict: 'The request could not be completed. Reload the ticket and try again.',
  },
  pl: {
    open: 'Otwarte', waiting: 'Oczekuje na Ciebie', title: 'Zgłoszenia pomocy',
    conflict: 'Nie udało się wykonać żądania. Odśwież zgłoszenie i spróbuj ponownie.',
  },
};

async function ticketForm(page, lang) {
  const subject = `Ticket ${randomUUID()}`;
  const body = `Question ${randomUUID()}`;
  expect((await page.goto(`/${lang}/support/tickets/create`)).status()).toBe(200);
  const form = page.locator('main form[action$="/support/tickets"]');
  await form.locator('[name=category]').selectOption('account');
  await form.locator('[name=subject]').fill(subject);
  await form.locator('[name=body]').fill(body);
  return { button: form.locator('button'), subject, body };
}

async function tickets(account) {
  return query('SELECT id, subject, status, lock_version::int FROM tickets WHERE account_id=$1', [account.id]);
}

async function messages(id) {
  return query(
    'SELECT author_kind, internal, body FROM ticket_messages WHERE ticket_id=$1 ORDER BY id',
    [id],
  );
}

async function expectTicket(page, lang, subject, state) {
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(copy[lang].title);
  await expect(page.getByRole('heading', { name: subject, exact: true })).toBeVisible();
  await expect(page.getByText(`Status: ${copy[lang][state]}`, { exact: false })).toBeVisible();
}

test('ticket creation preserves one ticket and one initial message across parallel submit, history and reload', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const owner = await signIn(page, lang);
  const { button, subject, body } = await ticketForm(page, lang);
  const original = await snapshot(button);
  expect(await tickets(owner)).toEqual([]);

  const [navigation, duplicate] = await Promise.all([
    submitForm(page, button, original.url),
    replay(page, original),
  ]);
  const rows = await tickets(owner);
  expect(rows).toEqual([{ id: expect.any(String), subject, status: 'open', lock_version: 1 }]);
  const [{ id }] = rows;
  const detail = `/${lang}/support/tickets/${id}`;
  expectRedirect(navigation, 302, detail);
  expectRedirect(duplicate, 302, detail);
  await expect(page).toHaveURL(detail);
  await expectTicket(page, lang, subject, 'open');

  await reloadGet(page);
  await expectTicket(page, lang, subject, 'open');
  await page.evaluate(() => history.back());
  await expect(page).toHaveURL(`/${lang}/support/tickets/create`);
  await page.evaluate(() => history.forward());
  await expect(page).toHaveURL(detail);
  await expectTicket(page, lang, subject, 'open');

  await page.evaluate(() => history.back());
  await expect(page).toHaveURL(`/${lang}/support/tickets/create`);
  await expect(button).toBeVisible();
  const restored = await snapshot(button);
  expect(
    new URLSearchParams(restored.fields).get('request_key') === new URLSearchParams(original.fields).get('request_key'),
    'browser history preserved the submitted request key',
  ).toBe(true);
  await expect(page.locator('main [name=subject]')).toHaveValue(subject);
  await expect(page.locator('main [name=body]')).toHaveValue(body);
  expectRedirect(await submitForm(page, button, restored.url), 302, detail);
  await expect(page).toHaveURL(detail);
  await expectTicket(page, lang, subject, 'open');
  expect(await tickets(owner)).toEqual(rows);
  expect(await messages(id)).toEqual([{ author_kind: 'user', internal: false, body }]);
});

test('ticket creation rejects tokens from signed-out, rotated and independent sessions', async ({ page, context, browser }, info) => {
  const lang = info.project.metadata.locale;
  const owner = await signIn(page, lang);
  const { button } = await ticketForm(page, lang);
  const loggedOutForm = await snapshot(button);
  const signedOut = await cookie(context);
  await login(page, lang, owner); // The shared helper signs out through the browser first.
  await expect(page).toHaveURL(`/${lang}/`);
  await expectSession(signedOut);
  await expectSession(await cookie(context), owner.id);
  await expectCsrfRefusal(await replay(page, loggedOutForm), lang);
  expect(await tickets(owner)).toEqual([]);

  const fresh = await ticketForm(page, lang);
  const beforeSudo = await snapshot(fresh.button);
  const regular = await cookie(context);
  expect((await page.goto(`/${lang}/account/sudo`)).status()).toBe(200);
  await submit(page, 'main form', { current_password: owner.password });
  await expect(page).toHaveURL(`/${lang}/password/change`);
  await expectSession(regular);
  await expectSession(await cookie(context), owner.id);
  await expectCsrfRefusal(await replay(page, beforeSudo), lang);
  expect(await tickets(owner)).toEqual([]);

  const independent = await otherBrowser(browser, info);
  try {
    const tab = await independent.newPage();
    await login(tab, lang, owner);
    await expect(tab).toHaveURL(`/${lang}/`);
    const secondSession = await cookie(independent);
    await expectSession(secondSession, owner.id);
    expect(secondSession !== await cookie(context), 'independent browser has its own session').toBe(true);
    const other = await ticketForm(tab, lang);
    await expectCsrfRefusal(await replay(page, await snapshot(other.button)), lang);
    expect(await tickets(owner)).toEqual([]);

    const current = await ticketForm(page, lang);
    const submission = await snapshot(current.button);
    const response = await submitForm(page, current.button, submission.url);
    const rows = await tickets(owner);
    expect(rows).toEqual([{ id: expect.any(String), subject: current.subject, status: 'open', lock_version: 1 }]);
    expectRedirect(response, 302, `/${lang}/support/tickets/${rows[0].id}`);
    await expectTicket(page, lang, current.subject, 'open');
  } finally {
    await independent.close();
  }
});

test('parallel staff reply and a stale second tab preserve one reply, audit, notification and mail', async ({ page, browser }, info) => {
  test.setTimeout(150_000);
  const lang = info.project.metadata.locale;
  const owner = await signIn(page, lang);
  expect((await page.goto(`/${lang}/account/notifications`)).status()).toBe(200);
  await submit(page, 'main form', { support_email: true, support_portal: true });
  const preference = await query(
    "SELECT email_enabled, portal_enabled FROM account_notification_prefs WHERE account_id=$1 AND category='support'",
    [owner.id],
  );
  expect(preference).toEqual([{ email_enabled: true, portal_enabled: true }]);
  const initial = await ticketForm(page, lang);
  const creation = await snapshot(initial.button);
  const opened = await submitForm(page, initial.button, creation.url);
  const [{ id }] = await tickets(owner);
  const detail = `/${lang}/support/tickets/${id}`;
  expectRedirect(opened, 302, detail);

  const staffContext = await otherBrowser(browser, info);
  try {
    const staff = await staffContext.newPage();
    const actor = await signIn(staff, lang, 'support');
    const second = await staffContext.newPage();
    const staffDetail = `/${lang}/admin/support/tickets/${id}`;
    const firstReply = 'The first staff response';
    const secondReply = 'A different staff response based on the old ticket';
    for (const [tab, message] of [[staff, firstReply], [second, secondReply]]) {
      expect((await tab.goto(staffDetail)).status()).toBe(200);
      await tab.locator('main form[action$="/reply"] [name=body]').fill(message);
      await expect(tab.locator('main form[action$="/reply"] [name=lock_version]')).toHaveValue('1');
    }
    const reply = staff.locator('main form[action$="/reply"] button');
    const original = await snapshot(reply);
    const [first, repeat] = await Promise.all([replay(staff, original), replay(staff, original)]);
    const [accepted, refused] = [first, repeat].sort((a, b) => a.status() - b.status());
    expectRedirect(accepted, 302, staffDetail);
    expect(refused.status()).toBe(409);
    expect(await refused.text()).toBe(copy[lang].conflict);
    const stale = second.locator('main form[action$="/reply"] button');
    const stalePost = await snapshot(stale);
    const conflict = await submitForm(second, stale, stalePost.url);
    expect(conflict.status()).toBe(409);
    await expect(second.locator('body')).toHaveText(copy[lang].conflict);

    expect((await page.goto(detail)).status()).toBe(200);
    await reloadGet(page);
    await expectTicket(page, lang, initial.subject, 'waiting');
    await expect(page.getByText(firstReply, { exact: true })).toBeVisible();
    expect(await tickets(owner)).toEqual([{ id, subject: initial.subject, status: 'waiting_user', lock_version: 2 }]);
    expect(await messages(id)).toEqual([
      { author_kind: 'user', internal: false, body: initial.body },
      { author_kind: 'staff', internal: false, body: firstReply },
    ]);
    const audits = await query(
      "SELECT actor FROM audit_log WHERE target=$1 AND action='support.ticket_replied'",
      [id],
    );
    expect(audits).toEqual([{ actor: actor.id }]);
    const notices = await query(
      "SELECT id FROM notifications WHERE account_id=$1 AND category='support' AND body LIKE '%' || $2 || '%'",
      [owner.id, id],
    );
    expect(notices).toHaveLength(1);
    await expect.poll(() => query(
      `SELECT e.delivered_at IS NOT NULL AS delivered, o.state
       FROM email_outbox e LEFT JOIN outbox o ON o.kind='notify.email' AND o.idempotency_key=e.id::text
       WHERE e.account_id=$1 AND e.category='support' AND e.text_body LIKE '%' || $2 || '%'`,
      [owner.id, id],
    ), { timeout: 90_000, message: 'one completed notification delivery job' }).toEqual([
      { delivered: true, state: 'done' },
    ]);
    const response = await page.request.get('/__mail');
    expect(response.status()).toBe(200);
    const mailbox = await response.json();
    expect(mailbox.filter(mail =>
      mail.to.includes(owner.email) && mail.raw.replace(/=\r?\n/g, '').includes(id)
    ).length).toBe(1);
  } finally {
    await staffContext.close();
  }
});
