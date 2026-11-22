import { randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { login, logout, password, register, seedPasswordRange, submit } from '../helpers.mjs';
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

function bearerLink(page, message, lang, kind) {
  const [, body] = copy[kind][lang === 'pl' ? 0 : 1];
  const [intro, candidate, ...extra] = message.text.split('\n\n');
  expect(intro === body && extra.length === 0, 'only the localized copy and one link').toBe(true);
  expect(message.headers.has('list-unsubscribe')).toBe(false);
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
  const next = password();
  await seedPasswordRange(next);
  await submit(page, 'main form', { email: account.email, password: next, password_confirmation: next });
  await expect(page).toHaveURL(/\/login$/);
  await login(page, lang, { ...account, password: next });
  await expect(page).not.toHaveURL(/\/login$/);
  await logout(page, lang);
  await page.goto(reset);
  const replay = password();
  await seedPasswordRange(replay);
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

test('security notices and the MFA fallback use the account language', async ({ page }, info) => {
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
  const next = password();
  await seedPasswordRange(next);
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
  const fallback = await securityMail(page, account.email, lang, 'mfa');
  const [intro, code, ...extra] = fallback.text.split('\n\n');
  expect(intro === copy.mfa[lang === 'pl' ? 0 : 1][1] && extra.length === 0).toBe(true);
  expect(/^[a-f0-9]{64}$/.test(code), 'MFA mail contains exactly one code').toBe(true);
  expect(fallback.headers.has('list-unsubscribe')).toBe(false);
  await submit(page, 'form[action$="/mfa/challenge"]', { code });
  await expect(page).not.toHaveURL(/\/mfa\/challenge$/);
  await login(page, other, updated);
  await expect(page).toHaveURL(/\/mfa\/challenge$/);
  await submit(page, 'form[action$="/mfa/challenge"]', { code });
  await expect(page).toHaveURL(/\/mfa\/challenge$/);
  await expect(page.locator('[role=alert]')).toBeVisible();
});

test('support replies, reports, sanctions and appeals deliver localized mail with unsubscribe', async ({ page, browser }, info) => {
  test.setTimeout(360_000);
  const lang = info.project.metadata.locale;
  const other = lang === 'pl' ? 'en' : 'pl';
  const account = await player(page, lang);
  const context = await browser.newContext({
    baseURL: new URL(page.url()).origin,
    ignoreHTTPSErrors: true, locale: other, viewport: info.project.use.viewport,
  });
  const staffPage = await context.newPage();
  const subject = lang === 'pl' ? 'Pomoc Oteryn' : 'Oteryn support';
  const replyCopy = lang === 'pl' ? 'Pomoc odpowiedziała na Twoje zgłoszenie.' : 'Support replied to your ticket.';
  const statusCopy = lang === 'pl' ? 'Zmienił się status Twojego zgłoszenia.' : 'Your support ticket status changed.';
  try {
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
    await page.goto(`/${lang}/account/notifications`);
    await submit(page, 'main form', { support_email: true, support_portal: true });
    await page.goto(`/${lang}/support/tickets/create`);
    const title = unique();
    await submit(page, 'main form', { subject: title, body: title });
    await expect(page).toHaveURL(/support\/tickets\/[a-f0-9-]+$/);
    const ticket = new URL(page.url()).pathname;
    const id = ticket.split('/').at(-1);
    expect((await staffPage.goto(`/admin/support/tickets/${id}`)).status()).toBe(200);
    const reply = unique();
    await action(staffPage, 'form:has(input[value=reply])', { body: reply });
    await expect(staffPage.locator('main')).toContainText(reply);
    await action(staffPage, 'form:has(input[value=ticket_status])', { status: 'resolved' });

    const messages = [];
    for (const copy of [replyCopy, statusCopy]) {
      const message = await mail(page, account.email, subject, copy, copy);
      const candidate = message.text.slice(`${subject}\n\n${copy} `.length);
      const target = absoluteLink(page, candidate, /^\/(?:en|pl)\/support\/tickets\/[a-f0-9-]+$/);
      expect(target.pathname).toBe(ticket);
      expect(target.search).toBe('');
      expect(message.text === `${subject}\n\n${copy} ${target.href}`, 'only localized copy and the ticket link').toBe(true);
      expect(message.parts.filter(part => part.type === 'text/html').length).toBe(1);
      expect((await page.goto(target.href)).status()).toBe(200);
      await expect(page.locator('main')).toContainText(reply);
      messages.push(message);
    }

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
    const reportCopy = `${lang === 'pl' ? 'Podjęto działanie' : 'Action taken'}: ${outcome}`;
    const reportNotice = await mail(page, account.email, subject, reportCopy, outcome);
    expect(reportNotice.text === `${subject}\n\n${reportCopy}`, 'only the localized public report outcome is mailed').toBe(true);
    messages.push(reportNotice);

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
    // Player appeal submission has no mounted route. Match the existing staff integration fixture.
    const requested = await query(
      "UPDATE support_appeal_workflows SET status='requested',message=$3 WHERE sanction_id=$1 AND owner_id=$2 AND status='none' RETURNING status",
      [sanction, account.id, unique()],
    );
    expect(requested).toEqual([{ status: 'requested' }]);
    await staffPage.reload();
    await action(staffPage, 'form:has(input[value=appeal])', { appeal_status: 'reviewing', appeal_outcome: '' });
    const reviewCopy = lang === 'pl' ? 'Odwołanie w trakcie rozpatrywania' : 'Appeal under review';
    const review = await mail(page, account.email, subject, reviewCopy, reviewCopy);
    expect(review.text === `${subject}\n\n${reviewCopy}`, 'only the localized appeal status is mailed').toBe(true);
    messages.push(review);
    const decision = unique();
    await action(staffPage, 'form:has(input[value=appeal])', { appeal_status: 'accepted', appeal_outcome: decision });
    const appealCopy = `${lang === 'pl' ? 'Odwołanie przyjęte' : 'Appeal accepted'}: ${decision}`;
    const appeal = await mail(page, account.email, subject, appealCopy, decision);
    expect(appeal.text === `${subject}\n\n${appealCopy}`, 'only the localized public appeal outcome is mailed').toBe(true);
    messages.push(appeal);

    for (const message of messages) {
      // One-click and email change consume the same ten-per-minute source budget.
      await expect.poll(async () => {
        const [usage] = await query(
          "SELECT COALESCE(max(count),0)::int AS used FROM web_rate_limits WHERE limiter_name='identity-anonymous-source' AND window_start=date_trunc('minute',clock_timestamp())",
        );
        return usage.used;
      }, { timeout: 65_000, intervals: [1000] }).toBeLessThanOrEqual(6);
      expect(message.headers.get('list-unsubscribe-post')).toBe('List-Unsubscribe=One-Click');
      const candidate = message.headers.get('list-unsubscribe')?.match(/^<(.*)>$/)?.[1];
      const target = absoluteLink(page, candidate, /^\/(?:en|pl)\/unsubscribe$/);
      expect([...target.searchParams.keys()]).toEqual(['token']);
      expect(Boolean(target.searchParams.get('token')), 'one signed unsubscribe capability').toBe(true);
      const response = await page.request.post(target.href, { form: { 'List-Unsubscribe': 'One-Click' } });
      expect(response.status()).toBe(200);
    }
    await page.goto(`/${lang}/account/notifications`);
    await expect(page.locator('[name=support_email]')).not.toBeChecked();
    await expect(page.locator('[name=support_portal]')).toBeChecked();
  } finally {
    await context.close();
  }
});

test('duplicate registration and referral invitations deliver localized, absolute destinations', async ({ page }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const other = lang === 'pl' ? 'en' : 'pl';
  const account = await register(page, lang);
  const attempt = password();
  await seedPasswordRange(attempt);
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
