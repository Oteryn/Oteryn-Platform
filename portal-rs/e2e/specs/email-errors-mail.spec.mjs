import { randomUUID } from 'node:crypto';
import { request as playwrightRequest } from '@playwright/test';
import { test, expect } from '../fixtures.mjs';
import { login, logout, register, seedPasswordRange, submit } from '../helpers.mjs';
import { action, enrollMfa, loginWithRecoveryCode, player, query, unique } from '../staff-rbac-helpers.mjs';

const copy = {
  verification: [
    ['Potwierdź adres e-mail', 'Potwierdź swój adres e-mail:'],
    ['Verify your email address', 'Verify your email address:'],
  ],
  reset: [
    ['Zresetuj hasło do Oteryn', 'Otwórz ten link w ciągu 60 minut, aby zresetować hasło. Jeśli nie prosisz o zmianę, zignoruj tę wiadomość.'],
    ['Reset your Oteryn password', 'Open this link within 60 minutes to reset your password. If you did not request this, ignore this message.'],
  ],
  device: [
    ['Nowe urządzenie', 'Logowanie z urządzenia:'],
    ['New device', 'Device login:'],
  ],
  password: [
    ['Zmieniono hasło', 'Hasło Twojego konta zostało zmienione.'],
    ['Password changed', 'Your account password has been changed.'],
  ],
  mfa: [
    ['Twój kod weryfikacyjny Oteryn', 'Wpisz ten kod wyłącznie na stronie logowania Oteryn. Kod wygasa za pięć minut. Zignoruj wiadomość, jeśli nie proszono o kod.'],
    ['Your Oteryn verification code', 'Enter this code only on the Oteryn sign-in page. It expires in five minutes. Ignore this email if you did not request it.'],
  ],
  confirm: [
    ['Bezpieczeństwo adresu e-mail', 'Potwierdź nowy adres e-mail. Bez klucza odzyskiwania zmiana nastąpi po siedmiu dniach.'],
    ['Email security', 'Confirm your new email address. Without a recovery key the change takes seven days.'],
  ],
  revert: [
    ['Bezpieczeństwo adresu e-mail', 'To nie ja: anuluj lub cofnij zmianę adresu e-mail. Link wygasa po 30 dniach.'],
    ['Email security', 'This was not me: cancel or revert this email change. This link expires after 30 days.'],
  ],
};

function decode(value, encoding) {
  if (/^base64$/i.test(encoding)) return Buffer.from(value, 'base64').toString('utf8');
  if (!/^quoted-printable$/i.test(encoding)) return value;
  const bytes = value.replace(/=\r?\n/g, '').replace(/=([\da-f]{2})/gi, (_, byte) =>
    String.fromCharCode(parseInt(byte, 16)));
  return Buffer.from(bytes, 'latin1').toString('utf8');
}

function mime(raw) {
  const separator = /\r?\n\r?\n/.exec(raw);
  const head = raw.slice(0, separator.index).replace(/\r?\n[ \t]+/g, ' ');
  const headers = new Map(head.split(/\r?\n/).map(line => {
    const colon = line.indexOf(':');
    return [line.slice(0, colon).toLowerCase(), line.slice(colon + 1).trim()];
  }));
  const content = raw.slice(separator.index + separator[0].length);
  const type = headers.get('content-type') || 'text/plain';
  const boundary = type.match(/boundary="?([^";\s]+)"?/i)?.[1];
  const parts = boundary ? content.split(`--${boundary}`).slice(1, -1)
    .flatMap(part => mime(part.replace(/^\r?\n/, '')).parts) : [{
    type: type.split(';')[0].toLowerCase(),
    contentType: headers.get('content-type'),
    text: decode(content, headers.get('content-transfer-encoding')).replace(/\r\n/g, '\n').trim(),
  }];
  const subject = headers.get('subject')?.replace(/(\?=) +(?==\?)/g, '$1')
    .replace(/=\?utf-8\?([bq])\?([^?]+)\?=/gi, (_, encoding, value) => encoding.toLowerCase() === 'b'
      ? decode(value, 'base64') : decode(value.replace(/_/g, ' '), 'quoted-printable'));
  return { headers, subject, parts };
}

async function mail(page, email, subject, body, discriminator = '') {
  let message;
  await expect.poll(async () => {
    const response = await page.request.get('/__mail');
    if (!response.ok()) return false;
    message = (await response.json()).filter(entry => entry.to.includes(email))
      .map(entry => mime(entry.raw))
      .find(entry => entry.subject === subject && entry.parts.some(part => part.text.includes(discriminator)));
    return Boolean(message);
  }, { message: `localized SMTP subject: ${subject}`, timeout: 90_000 }).toBe(true);
  const plain = message.parts.filter(part => part.type === 'text/plain');
  expect(plain.length).toBe(1);
  expect(plain[0].text.includes(body), 'plain-text body matches the account locale').toBe(true);
  return { ...message, text: plain[0].text };
}

function securityMail(page, email, lang, kind) {
  const [subject, body] = copy[kind][lang === 'pl' ? 0 : 1];
  return mail(page, email, subject, body, body);
}

function absoluteLink(page, candidate, path) {
  expect(Boolean(candidate) && /^https?:\/\//.test(candidate), 'absolute email link').toBe(true);
  const target = new URL(candidate);
  expect(target.origin === new URL(process.env.OTERYN_PUBLIC_ORIGIN || page.url()).origin).toBe(true);
  expect(Boolean(target.username || target.password || target.hash), 'link excludes additional credentials').toBe(false);
  expect(path.test(target.pathname), 'expected localized link destination').toBe(true);
  return target;
}

function assertPlainMime(message) {
  expect(message.headers.get('mime-version')).toBe('1.0');
  expect(message.parts.find(part => part.type === 'text/plain').contentType)
    .toMatch(/^text\/plain;\s*charset="?utf-8"?$/i);
}

function bearerLink(page, message, lang, kind) {
  const [, body] = copy[kind][lang === 'pl' ? 0 : 1];
  const [intro, candidate, ...extra] = message.text.split('\n\n');
  expect(intro === body && extra.length === 0, 'only the localized copy and one link').toBe(true);
  expect(message.headers.has('list-unsubscribe')).toBe(false);
  if (kind === 'verification' || kind === 'reset') {
    assertPlainMime(message);
  }
  const path = {
    verification: new RegExp(`^/${lang}/verify-email$`),
    reset: new RegExp(`^/${lang}/reset-password/[a-f0-9]{64}$`),
    confirm: /^\/email-change\/confirm\/[a-f0-9]{64}$/,
    revert: /^\/email-change\/revert\/[a-f0-9]{64}$/,
  };
  const target = absoluteLink(page, candidate, path[kind]);
  expect([...target.searchParams.keys()]).toEqual(kind === 'verification' ? ['token'] : []);
  if (kind === 'verification') {
    expect(/^[a-f0-9]{64}$/.test(target.searchParams.get('token')), 'single verification token').toBe(true);
  }
  return target.href;
}

test('verification and reset mail have localized copy and single-use links', async ({ page }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const other = lang === 'pl' ? 'en' : 'pl';
  const account = await register(page, lang);
  const verification = bearerLink(page, await securityMail(page, account.email, lang, 'verification'), lang, 'verification');
  expect((await page.goto(verification)).status()).toBe(422);
  await expect(page.locator('main form[action*="verify-email?token="]')).toHaveCount(0);

  await page.goto(`/${other}/forgot-password`);
  await submit(page, 'main form', { email: account.email });
  const reset = bearerLink(page, await securityMail(page, account.email, lang, 'reset'), lang, 'reset');
  expect((await page.goto(reset)).status()).toBe(200);
  const next = await seedPasswordRange();
  await submit(page, 'main form', { email: account.email, password: next, password_confirmation: next });
  await expect(page).toHaveURL(/\/login$/);
  await login(page, lang, { ...account, password: next });
  await expect(page).not.toHaveURL(/\/login$/);
  await logout(page, lang);
  await page.goto(reset);
  const replay = await seedPasswordRange('alternate');
  await submit(page, 'main form', { email: account.email, password: replay, password_confirmation: replay });
  await expect(page.locator('[role=alert]')).toBeVisible();
});

test('email change delivers separate confirmation and reversal capabilities', async ({ page }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const other = lang === 'pl' ? 'en' : 'pl';
  const account = await register(page, lang);
  await login(page, lang, account);
  await page.goto(`/${lang}/account/recovery-key`);
  await submit(page, 'main form', { current_password: account.password });
  const proof = (await page.locator('.op-code').innerText()).trim();
  await submit(page, 'main form', { current_password: account.password, recovery_key: proof });
  const email = `changed-${randomUUID()}@local.test`;
  await page.goto(`/${other}/account/security/email`);
  await submit(page, 'main form', {
    email, email_confirmation: email, current_password: account.password, proof, recovery: true,
  });
  const confirm = bearerLink(page, await securityMail(page, email, lang, 'confirm'), lang, 'confirm');
  const revert = bearerLink(page, await securityMail(page, account.email, lang, 'revert'), lang, 'revert');
  expect(confirm.split('/').at(-1) !== revert.split('/').at(-1), 'distinct capabilities').toBe(true);
  const [{ id }] = await query('SELECT id FROM accounts WHERE email=$1', [account.email]);
  const state = async () => (await query('SELECT email,native_security_generation FROM accounts WHERE id=$1', [id]))[0];
  const before = await state();
  await page.goto(confirm);
  await submit(page);
  expect(await state()).toEqual({ email, native_security_generation: String(Number(before.native_security_generation) + 1) });
  await page.goto(revert);
  await submit(page);
  const restored = await state();
  expect(restored.email).toBe(account.email);
  for (const link of [confirm, revert]) {
    await page.goto(link);
    await submit(page);
    expect(await state()).toEqual(restored);
  }
});

test('security notices use the account language and the existing MFA fallback rejects replay', async ({ page }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const other = lang === 'pl' ? 'en' : 'pl';
  const account = await register(page, lang);
  await login(page, other, account);
  const device = await securityMail(page, account.email, lang, 'device');
  expect(device.headers.has('list-unsubscribe')).toBe(false);
  expect(/https?:\/\/|\b[a-f0-9]{64}\b/.test(device.text), 'device notice has no link or token').toBe(false);
  await page.goto(`/${other}/account/sudo`);
  await submit(page, 'main form', { current_password: account.password });
  await page.goto(`/${other}/password/change`);
  const next = await seedPasswordRange();
  await submit(page, 'main form', {
    current_password: account.password, password: next, password_confirmation: next,
  });
  await expect(page).toHaveURL(/\/login$/);
  const changed = await securityMail(page, account.email, lang, 'password');
  const [subject, body] = copy.password[lang === 'pl' ? 0 : 1];
  expect(changed.text === `${subject}\n\n${body}`, 'password notice contains only public copy').toBe(true);
  expect(changed.headers.has('list-unsubscribe')).toBe(false);

  const updated = { ...account, password: next };
  await login(page, other, updated);
  await enrollMfa(page, other, updated);
  await expect(page.locator("textarea[readonly]").first()).toBeVisible();
  await login(page, other, updated);
  await expect(page).toHaveURL(/\/mfa\/challenge$/);
  await submit(page, 'form[action$="/mfa/email"]');
  const fallback = await securityMail(page, account.email, other, 'mfa');
  const [intro, code, ...extra] = fallback.text.split('\n\n');
  expect(intro === copy.mfa[other === 'pl' ? 0 : 1][1] && extra.length === 0).toBe(true);
  expect(/^[a-f0-9]{64}$/.test(code), 'the existing MFA challenge supplies exactly one code').toBe(true);
  expect(fallback.headers.has('list-unsubscribe')).toBe(false);
  await submit(page, 'form[action$="/mfa/challenge"]', { code });
  await expect(page).not.toHaveURL(/\/mfa\/challenge$/);
  await login(page, other, updated);
  await expect(page).toHaveURL(/\/mfa\/challenge$/);
  await submit(page, 'form[action$="/mfa/challenge"]', { code });
  await expect(page).toHaveURL(/\/mfa\/challenge$/);
  await expect(page.locator('[role=alert]')).toBeVisible();
});

async function enableSupport(page, lang) {
  await page.goto(`/${lang}/account/notifications`);
  await submit(page, 'main form', { support_email: true, support_portal: true, news_email: true });
}

async function createSupportTicket(page, lang) {
  await page.goto(`/${lang}/support/tickets/create`);
  const title = unique();
  await submit(page, 'main form', { subject: title, body: title });
  await expect(page).toHaveURL(/support\/tickets\/[a-f0-9-]+$/);
  const ticket = new URL(page.url()).pathname;
  return { ticket, id: ticket.split('/').at(-1) };
}

function unsubscribeLink(page, message) {
  expect(message.headers.get('list-unsubscribe-post')).toBe('List-Unsubscribe=One-Click');
  const candidate = message.headers.get('list-unsubscribe')?.match(/^<(.*)>$/)?.[1];
  const target = absoluteLink(page, candidate, /^\/(?:en|pl)\/unsubscribe$/);
  expect([...target.searchParams.keys()]).toEqual(['token']);
  expect(Boolean(target.searchParams.get('token')), 'one signed unsubscribe capability').toBe(true);
  return target;
}

async function assertSupportUnsubscribe(page, support, messages) {
  const { account, staff, staffPage, lang, other } = support;
  const [staffAccount] = await query('SELECT id FROM accounts WHERE email=$1', [staff.email]);
  await staffPage.goto(`/${other}/account/notifications`);
  await submit(staffPage, 'main form', { support_email: true, support_portal: true });
  const preferences = async () => {
    const [row] = await query(
      `SELECT
        (SELECT email_enabled FROM account_notification_prefs WHERE account_id=$1 AND category='support') AS support,
        (SELECT email_enabled FROM account_notification_prefs WHERE account_id=$1 AND category='news') AS news,
        (SELECT portal_enabled FROM account_notification_prefs WHERE account_id=$1 AND category='support') AS portal,
        (SELECT email_enabled FROM account_notification_prefs WHERE account_id=$2 AND category='support') AS other_support`,
      [account.id, staffAccount.id],
    );
    return row;
  };
  for (const message of messages) {
    // One-click and email change consume the same ten-per-minute source budget.
    await expect.poll(async () => {
      const [usage] = await query(
        "SELECT COALESCE(max(count),0)::int AS used FROM web_rate_limits WHERE limiter_name='identity-anonymous-source' AND window_start=date_trunc('minute',clock_timestamp())",
      );
      return usage.used;
    }, { timeout: 65_000, intervals: [1000] }).toBeLessThanOrEqual(6);
    const target = unsubscribeLink(page, message);
    await enableSupport(page, lang);
    const enabled = { support: true, news: true, portal: true, other_support: true };
    await expect.poll(preferences).toEqual(enabled);
    const anonymous = await playwrightRequest.newContext();
    try {
      expect((await anonymous.storageState()).cookies).toEqual([]);
      const response = await anonymous.post(target.href, {
        form: { 'List-Unsubscribe': 'One-Click' }, maxRedirects: 0,
      });
      expect(response.status()).toBe(200);
      expect(await preferences()).toEqual({ ...enabled, support: false });
    } finally {
      await anonymous.dispose();
    }
  }
  await page.goto(`/${lang}/account/notifications`);
  await expect(page.locator('[name=support_email]')).not.toBeChecked();
  await expect(page.locator('[name=support_portal]')).toBeChecked();
}

const supportTest = test.extend({
  support: async ({ page, browser }, use, info) => {
    const lang = info.project.metadata.locale;
    const other = lang === 'pl' ? 'en' : 'pl';
    const account = await player(page, lang);
    const context = await browser.newContext({
      baseURL: new URL(page.url()).origin,
      locale: other, viewport: info.project.use.viewport,
    });
    try {
      const staffPage = await context.newPage();
      const staff = await register(staffPage, other);
      await query(
        "INSERT INTO account_roles(account_id,role_id) SELECT a.id,r.id FROM accounts a CROSS JOIN roles r WHERE a.email=$1 AND r.key='platform_admin'",
        [staff.email],
      );
      await login(staffPage, other, staff);
      await enrollMfa(staffPage, other, staff);
      const recovery = staffPage.locator("textarea[readonly]").first();
      await expect(recovery).toBeVisible();
      await loginWithRecoveryCode(staffPage, other, staff, await recovery.inputValue());
      await enableSupport(page, lang);
      const subject = lang === 'pl' ? 'Pomoc Oteryn' : 'Oteryn support';
      const replyCopy = lang === 'pl' ? 'Pomoc odpowiedziała na Twoje zgłoszenie.' : 'Support replied to your ticket.';
      const statusCopy = lang === 'pl' ? 'Zmienił się status Twojego zgłoszenia.' : 'Your support ticket status changed.';
      const [heading, description, openForm] = lang === 'pl'
        ? ['Konflikt zmian', 'Ten formularz jest nieaktualny.', 'Otwórz aktualny formularz']
        : ['Update conflict', 'This form is out of date.', 'Open the latest form'];
      const stateCopy = lang === 'pl'
        ? 'Ta akcja jest niedostępna w bieżącym stanie zgłoszenia.'
        : 'This action is unavailable in the ticket’s current state.';
      await use({ account, staff, staffPage, context, lang, other, subject, replyCopy, statusCopy,
        heading, description, openForm, stateCopy });
    } finally {
      await context.close();
    }
  },
});

supportTest.describe('support email and ticket errors', () => {
  supportTest.describe.configure({ timeout: 240_000 });

  supportTest('ticket updates deliver localized mail and stale forms link to the current ticket', async ({ page, support }) => {
    const { account, staffPage, context, lang, subject, replyCopy, statusCopy,
      heading, description, openForm, stateCopy } = support;
    const { ticket, id } = await createSupportTicket(page, lang);
    const denied = await page.request.post(`${ticket}/reply`, {
      headers: { accept: 'text/html', origin: new URL(page.url()).origin },
      form: { body: 'Missing CSRF token' },
      maxRedirects: 0,
    });
    expect(denied.status()).toBe(403);
    expect(denied.headers()['content-type']).toMatch(/^text\/html\b/);
    expect(denied.headers()['x-robots-tag']).toBe('noindex, nofollow');
    const staffTicket = `/${lang}/admin/support/tickets/${id}`;
    const localizedStaff = await context.newPage();
    expect((await localizedStaff.goto(staffTicket)).status()).toBe(200);
    const replySelector = 'form[action$="/reply"]';
    const localizedForms = [];
    for (const [browserPage, path] of [[page, ticket], [localizedStaff, staffTicket]]) {
      const form = browserPage.locator(replySelector);
      await expect(form).toHaveAttribute('action', `${path}/reply`);
      localizedForms.push({ browserPage, path, fields: await form.evaluate(
        element => Object.fromEntries(new FormData(element)),
      ) });
    }
    expect((await staffPage.goto(`/admin/support/tickets/${id}`)).status()).toBe(200);
    const replyForm = staffPage.locator('form:has(input[value=reply])');
    const stale = await replyForm.evaluate(form => Object.fromEntries(new FormData(form)));
    const reply = unique();
    await action(staffPage, 'form:has(input[value=reply])', { body: reply });
    await expect(staffPage.locator('main')).toContainText(reply);
    expect(await staffPage.locator('form:has(input[value=reply]) [name=lock_version]').inputValue())
      .not.toBe(stale.lock_version);
    const staleBody = unique();
    const conflict = await staffPage.request.post(`/admin/support/tickets/${id}`, {
      headers: { accept: 'text/html', 'accept-language': lang, origin: new URL(staffPage.url()).origin },
      form: { ...stale, body: staleBody },
      maxRedirects: 0,
    });
    expect(conflict.status()).toBe(409);
    expect(conflict.headers()['content-language']).toBe(lang);
    expect(conflict.headers()['cache-control']).toBe('no-store');
    const conflictHtml = await conflict.text();
    expect(conflictHtml).toContain(`<h1>${heading}</h1>`);
    expect(conflictHtml).toContain(`<p>${description}</p>`);
    expect(conflictHtml).not.toContain(staleBody);
    await staffPage.setContent(conflictHtml);
    const recoveryLink = staffPage.getByRole('link', { name: openForm, exact: true });
    await expect(recoveryLink).toHaveAttribute('href', `/admin/support/tickets/${id}`);
    const [current] = await Promise.all([
      staffPage.waitForResponse(response =>
        new URL(response.url()).pathname === `/admin/support/tickets/${id}`
        && response.request().method() === 'GET'),
      recoveryLink.click(),
    ]);
    expect(current.status()).toBe(200);
    await expect(staffPage.locator('form:has(input[value=reply]) [name=lock_version]'))
      .not.toHaveValue(stale.lock_version);
    for (const { browserPage, path, fields } of localizedForms) {
      const attempted = unique();
      const refusal = await browserPage.request.post(`${path}/reply`, {
        headers: {
          accept: 'text/html;q=1, application/json;q=0.1',
          origin: new URL(browserPage.url()).origin,
        },
        form: { ...fields, body: attempted },
        maxRedirects: 0,
      });
      expect(refusal.status()).toBe(409);
      expect(refusal.headers()['content-type']).toMatch(/^text\/html\b/);
      expect(refusal.headers()['content-language']).toBe(lang);
      expect(refusal.headers()['cache-control']).toBe('no-store');
      const html = await refusal.text();
      expect(html).toContain(`<h1>${heading}</h1>`);
      expect(html).toContain(`<p>${description}</p>`);
      expect(html).not.toContain(attempted);
      await browserPage.setContent(html);
      const link = browserPage.getByRole('link', { name: openForm, exact: true });
      await expect(link).toHaveAttribute('href', path);
      const [latestForm] = await Promise.all([
        browserPage.waitForResponse(response =>
          new URL(response.url()).pathname === path && response.request().method() === 'GET'),
        link.click(),
      ]);
      expect(latestForm.status()).toBe(200);
      await expect(browserPage.locator(`${replySelector} [name=lock_version]`))
        .not.toHaveValue(fields.lock_version);
      await expect(browserPage.locator('main')).toContainText(reply);
      await expect(browserPage.locator('main')).not.toContainText(attempted);
    }
    await action(staffPage, 'form:has(input[value=ticket_status])', { status: 'resolved' });
    const ticketSnapshot = async () => (await query(
      `SELECT status, lock_version::text,
              (SELECT count(*)::int FROM ticket_messages WHERE ticket_id=tickets.id) AS messages
       FROM tickets WHERE id=$1`, [id],
    ))[0];
    await expect.poll(ticketSnapshot).toMatchObject({ status: 'resolved' });
    const resolved = await ticketSnapshot();
    expect(resolved.status).toBe('resolved');
    await expect(replyForm.locator('[name=lock_version]')).toHaveValue(resolved.lock_version);
    const latest = await replyForm.evaluate(form => Object.fromEntries(new FormData(form)));
    const refusedReply = unique();
    const stateConflict = await staffPage.request.post(`/admin/support/tickets/${id}`, {
      headers: { accept: 'text/html', 'accept-language': lang, origin: new URL(staffPage.url()).origin },
      form: { ...latest, body: refusedReply },
      maxRedirects: 0,
    });
    expect(stateConflict.status()).toBe(409);
    expect(stateConflict.headers()['content-language']).toBe(lang);
    expect(stateConflict.headers()['cache-control']).toBe('no-store');
    const stateHtml = await stateConflict.text();
    expect(stateHtml).toContain(`<h1>${heading}</h1>`);
    expect(stateHtml).toContain(`<p>${stateCopy}</p>`);
    expect(stateHtml).not.toContain(description);
    expect(stateHtml).not.toContain(openForm);
    expect(stateHtml).not.toContain(refusedReply);
    expect(await ticketSnapshot()).toEqual(resolved);

    const unsupported = await staffPage.request.post(`/admin/support/tickets/${id}`, {
      headers: {
        accept: 'text/html', 'accept-language': lang, origin: new URL(staffPage.url()).origin,
        'content-type': 'text/plain', 'x-csrf-token': latest._csrf,
      },
      data: refusedReply, maxRedirects: 0,
    });
    expect(unsupported.status()).toBe(415);
    expect(unsupported.headers()['content-type']).toMatch(/^text\/html\b/);
    expect(unsupported.headers()['content-language']).toBe(lang);
    expect(unsupported.headers()['cache-control']).toBe('no-store');
    const fallbackHtml = await unsupported.text();
    const [fallbackTitle, fallbackDescription] = lang === 'pl'
      ? ['Nieobsługiwany format żądania', 'Wyślij formularz ze strony i spróbuj ponownie.']
      : ['Unsupported request format', 'Submit the form from the page and try again.'];
    expect(fallbackHtml).toContain(`<h1>${fallbackTitle}</h1>`);
    expect(fallbackHtml).toContain(`<p>${fallbackDescription}</p>`);
    expect(fallbackHtml).not.toContain(refusedReply);
    expect(await ticketSnapshot()).toEqual(resolved);

    for (const copy of [replyCopy, statusCopy]) {
      const message = await mail(page, account.email, subject, copy, copy);
      const candidate = message.text.slice(`${subject}\n\n${copy} `.length);
      expect(candidate === ticket, 'ticket mail uses the exact localized relative path').toBe(true);
      expect(message.text === `${subject}\n\n${copy} ${ticket}`, 'only localized copy and the ticket path').toBe(true);
      expect(message.parts.filter(part => part.type === 'text/html').length).toBe(1);
      expect((await page.goto(ticket)).status()).toBe(200);
      await expect(page.locator('main')).toContainText(reply);
      unsubscribeLink(page, message);
    }

    await action(staffPage, 'form:has(input[value=ticket_status])', { status: 'open' });
    await expect.poll(ticketSnapshot).toMatchObject({ status: 'open' });
    expect((await page.goto(ticket)).status()).toBe(200);
    const draftForm = page.locator(replySelector);
    await expect(draftForm).toBeVisible();
    const draft = await draftForm.evaluate(form => Object.fromEntries(new FormData(form)));
    await action(staffPage, 'form:has(input[value=ticket_status])', { status: 'closed' });
    await expect.poll(ticketSnapshot).toMatchObject({ status: 'closed' });
    const closed = await ticketSnapshot();
    expect(closed.lock_version).not.toBe(draft.lock_version);
    const draftBody = unique();
    const closedRefusal = await page.request.post(`${ticket}/reply`, {
      headers: { accept: 'text/html', origin: new URL(page.url()).origin },
      form: { ...draft, body: draftBody },
      maxRedirects: 0,
    });
    expect(closedRefusal.status()).toBe(409);
    expect(closedRefusal.headers()['content-language']).toBe(lang);
    const closedHtml = await closedRefusal.text();
    expect(closedHtml).toContain(`<p>${stateCopy}</p>`);
    expect(closedHtml).not.toContain(description);
    expect(closedHtml).not.toContain(openForm);
    expect(closedHtml).not.toContain(draftBody);
    expect(await ticketSnapshot()).toEqual(closed);
    expect((await page.goto(ticket)).status()).toBe(200);
    await expect(draftForm).toHaveCount(0);
  });

  supportTest('moderation decisions send report, sanction and appeal mail', async ({ page, support }) => {
    supportTest.setTimeout(300_000);
    const { account, staffPage, lang, subject, heading, description, openForm, stateCopy } = support;
    await page.goto(`/${lang}/support/reports/create`);
    const evidence = unique();
    await submit(page, 'main form', { target_reference: account.id, evidence_summary: evidence });
    await expect(page).toHaveURL(/support\/reports\/[a-f0-9-]+$/);
    const report = new URL(page.url()).pathname.split('/').at(-1);
    expect((await staffPage.goto(`/admin/moderation/reports/${report}`)).status()).toBe(200);
    const outcome = unique();
    await action(staffPage, 'form:has(input[value=moderate])', {
      status: 'actioned', public_outcome: outcome, moderator_notes: unique(),
    });
    const reportSnapshot = async () => (await query(
      'SELECT status, lock_version::text, public_outcome FROM player_reports WHERE id=$1', [report],
    ))[0];
    await expect.poll(reportSnapshot).toMatchObject({ status: 'actioned', public_outcome: outcome });
    const actioned = await reportSnapshot();
    const reportForm = staffPage.locator('form:has(input[value=moderate])');
    await expect(reportForm.locator('[name=lock_version]')).toHaveValue(actioned.lock_version);
    const reportFields = await reportForm.evaluate(form => Object.fromEntries(new FormData(form)));
    const neutralConflict = await staffPage.request.post(`/admin/moderation/reports/${report}`, {
      headers: { accept: 'text/html', 'accept-language': lang, origin: new URL(staffPage.url()).origin },
      form: { ...reportFields, status: 'reviewing', public_outcome: '', moderator_notes: '' },
      maxRedirects: 0,
    });
    expect(neutralConflict.status()).toBe(409);
    const neutralHtml = await neutralConflict.text();
    const neutralCopy = lang === 'pl'
      ? 'Ta akcja jest sprzeczna z bieżącym stanem.'
      : 'This action conflicts with the current state.';
    expect(neutralHtml).toContain(`<h1>${heading}</h1>`);
    expect(neutralHtml).toContain(`<p>${neutralCopy}</p>`);
    expect(neutralHtml).not.toContain(description);
    expect(neutralHtml).not.toContain(openForm);
    expect(neutralHtml).not.toContain(stateCopy);
    expect(await reportSnapshot()).toEqual(actioned);
    const reportCopy = `actioned: ${outcome}`;
    const reportNotice = await mail(page, account.email, subject, reportCopy, outcome);
    expect(reportNotice.text === `${subject}\n\n${reportCopy}`, 'only the public report outcome is mailed').toBe(true);
    const messages = [reportNotice];

    const reason = unique();
    await staffPage.goto('/admin/moderation/enforcement/create');
    const now = Math.floor(Date.now() / 1000);
    const values = {
      target: account.id, category: 'restriction', status: 'active', public_reason: reason,
      moderator_notes: unique(), effective_at: String(now - 1), expires_at: String(now + 3600),
    };
    await action(staffPage, 'form:has(input[value=sanction])', values);
    await expect(staffPage).toHaveURL(/enforcement\/[a-f0-9-]+$/);
    const lift = unique();
    await action(staffPage, 'form:has(input[value=sanction])', { ...values, status: 'revoked', public_reason: lift });
    for (const marker of [reason, lift]) {
      const message = await mail(page, account.email, subject, marker, marker);
      expect(message.text === `${subject}\n\n${marker}`, 'sanction mail contains only the public reason').toBe(true);
      messages.push(message);
    }

    await staffPage.goto('/admin/moderation/enforcement/create');
    await action(staffPage, 'form:has(input[value=sanction])', { ...values, public_reason: unique() });
    await expect(staffPage).toHaveURL(/enforcement\/[a-f0-9-]+$/);
    const sanction = new URL(staffPage.url()).pathname.split('/').at(-1);
    const appealPath = `/${lang}/support/appeals/${sanction}`;
    const message = unique();
    expect((await page.goto(appealPath)).status()).toBe(200);
    await submit(page, 'main form', { message });
    await expect(page.locator('main')).toContainText(message);
    const appealState = () => query(
      'SELECT status, message FROM support_appeal_workflows WHERE sanction_id=$1 AND owner_id=$2',
      [sanction, account.id],
    );
    expect(await appealState()).toEqual([{ status: 'requested', message }]);

    await staffPage.reload();
    // The staff form offers final decisions while the endpoint still accepts the legacy review transition.
    const reviewForm = staffPage.locator('form:has(input[value=appeal])');
    const reviewFields = await reviewForm.evaluate(form => Object.fromEntries(new FormData(form)));
    const staffPath = new URL(staffPage.url()).pathname;
    const reviewResponse = await staffPage.request.post(staffPath, {
      headers: { origin: new URL(staffPage.url()).origin, connection: 'close' },
      form: { ...reviewFields, appeal_status: 'reviewing', appeal_outcome: '' },
      maxRedirects: 0,
    });
    expect(reviewResponse.status()).toBe(303);
    expect(await appealState()).toEqual([{ status: 'reviewing', message }]);
    await staffPage.reload();
    const reviewCopy = lang === 'pl' ? 'Odwołanie w trakcie rozpatrywania:' : 'Appeal under review:';
    const review = await mail(page, account.email, subject, reviewCopy, reviewCopy);
    expect(review.text === `${subject}\n\n${reviewCopy}`, 'only the appeal status is mailed').toBe(true);
    messages.push(review);
    const decision = unique();
    await action(staffPage, 'form:has(input[value=appeal])', { appeal_status: 'accepted', appeal_outcome: decision });
    const decisionLabel = lang === 'pl' ? 'Odwołanie uwzględnione' : 'Appeal accepted';
    const appealCopy = `${decisionLabel}: ${decision}`;
    const appeal = await mail(page, account.email, subject, appealCopy, decision);
    expect(appeal.text === `${subject}\n\n${appealCopy}`, 'only the public appeal outcome is mailed').toBe(true);
    messages.push(appeal);

    await assertSupportUnsubscribe(page, support, messages);
  });

  supportTest('signed support unsubscribe works without a session and isolates account preferences', async ({ page, support }) => {
    const { account, staffPage, lang, subject, replyCopy, statusCopy } = support;
    const { id } = await createSupportTicket(page, lang);
    expect((await staffPage.goto(`/admin/support/tickets/${id}`)).status()).toBe(200);
    const reply = unique();
    await action(staffPage, 'form:has(input[value=reply])', { body: reply });
    await expect(staffPage.locator('main')).toContainText(reply);
    await action(staffPage, 'form:has(input[value=ticket_status])', { status: 'resolved' });

    const messages = [];
    for (const copy of [replyCopy, statusCopy]) {
      messages.push(await mail(page, account.email, subject, copy, copy));
    }

    await assertSupportUnsubscribe(page, support, messages);
  });
});

test('duplicate registration and referral invitations deliver localized, absolute destinations', async ({ page }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const other = lang === 'pl' ? 'en' : 'pl';
  const account = await register(page, lang);
  const attempt = await seedPasswordRange();
  await page.goto(`/${other}/register`);
  await page.locator('[name=work_token]').waitFor({ state: 'attached' });
  await submit(page, 'main form', {
    email: account.email, password: attempt, password_confirmation: attempt, terms: true, privacy: true,
  });
  await expect(page).toHaveURL(new RegExp(`/${other}/register/check-inbox$`), { timeout: 90_000 });
  const [existingSubject, existingCopy] = lang === 'pl' ? [
    'Konto już istnieje',
    'Ten adres e-mail jest już powiązany z kontem. Jeśli to Ty próbujesz się zarejestrować, zaloguj się lub odzyskaj hasło.',
  ] : [
    'Account already exists',
    'This email address already belongs to an account. If you tried to register, sign in or reset your password.',
  ];
  const existing = await mail(page, account.email, existingSubject, existingCopy);
  assertPlainMime(existing);
  const destination = absoluteLink(page, existing.text.split('\n\n').at(-1), new RegExp(`^/${lang}/login$`));
  expect(existing.text === `${existingCopy}\n\n${destination.href}`, 'login link contains only the public destination').toBe(true);
  expect(existing.headers.has('list-unsubscribe')).toBe(false);
  expect((await page.goto(destination.href)).status()).toBe(200);

  await login(page, other, account);
  await page.goto(`/${other}/account/referrals`);
  const invitee = `invited-${randomUUID()}@local.test`;
  await submit(page, 'form:has(#new-email)', { email: invitee });
  const [inviteSubject, inviteCopy] = lang === 'pl' ? [
    'Zaproszenie do Oteryn', 'Gracz zaprosił Cię do Oteryn. Otwórz ten link w przeglądarce, aby kontynuować:',
  ] : [
    'You are invited to Oteryn', 'A player invited you to Oteryn. Open this link in your browser to continue:',
  ];
  const invitation = await mail(page, invitee, inviteSubject, inviteCopy);
  const target = absoluteLink(page, invitation.text.split('\n\n').at(-1),
    new RegExp(`^/${lang}/referrals/[a-f0-9-]{36}$`));
  expect(invitation.text === `${inviteCopy}\n\n${target.href}` && !target.search, 'only the invitation capability is present').toBe(true);
  expect(invitation.headers.has('list-unsubscribe')).toBe(false);
  await logout(page, lang);
  expect((await page.goto(target.href)).status()).toBe(200);
  await submit(page, 'main form');
  await expect(page).toHaveURL(new RegExp(`/${lang}/register$`));
});
