import { readFileSync } from 'node:fs';
import { SMTPServer } from 'smtp-server';
import { submit, login } from '../helpers.mjs';
import { seededPlayer } from '../outbox-helpers.mjs';
import { enqueue, expect, query, status, test, waitForJob } from './worker.mjs';

function catcher(env, accept) {
  return new SMTPServer({
    cert: readFileSync(env.OTERYN_INTERNAL_TLS_CERT_FILE),
    key: readFileSync(env.OTERYN_INTERNAL_TLS_KEY_FILE),
    onAuth(auth, _session, callback) {
      if (auth.username === env.OTERYN_SMTP_USERNAME && auth.password === env.OTERYN_SMTP_PASSWORD) {
        callback(null, { user: auth.username });
      } else {
        callback(new Error('SMTP authentication rejected'));
      }
    },
    onData(stream, session, callback) {
      const chunks = [];
      stream.on('data', chunk => chunks.push(chunk));
      stream.once('error', callback);
      stream.once('end', () => accept({
        to: session.envelope.rcptTo.map(recipient => recipient.address),
        raw: Buffer.concat(chunks).toString(),
      }, callback));
    },
  });
}

function listen(server, port) {
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, '127.0.0.1', () => resolve(server.server.address().port));
  });
}

const close = server => new Promise(resolve => server.close(resolve));

test('email change mail backs off, quarantines poison and drains a concurrent SMTP send', async ({ page, jobs }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seededPlayer(page, lang);
  const nextEmail = account.email.replace('outbox-', 'changed-');
  await page.goto(`/${lang}/account/recovery-key`);
  await submit(page, 'main form', { current_password: account.password });
  const recoveryKey = (await page.locator('.op-code').innerText()).trim();
  await submit(page, 'main form', { current_password: account.password, recovery_key: recoveryKey });
  const emailPath = `/${lang}/account/security/email`;
  const changeInput = {
    email: nextEmail, email_confirmation: nextEmail, current_password: account.password,
    proof: recoveryKey, recovery: true,
  };
  const mismatchedEmail = account.email.replace('outbox-', 'mismatched-');
  const viewport = page.viewportSize();
  await page.setViewportSize({ width: 390, height: 900 });
  expect((await page.goto(emailPath)).status()).toBe(200);
  const refusal = page.waitForResponse(response => response.request().method() === 'POST'
    && new URL(response.url()).pathname === emailPath && response.status() !== 403);
  await submit(page, 'main form', { ...changeInput, email_confirmation: mismatchedEmail });
  const refused = await refusal;
  expect(refused.status()).toBe(302);
  expect(refused.headers().location).toBe(emailPath);
  await expect(page.getByRole('status')).toHaveText(lang === 'pl'
    ? 'Odmowa lub nieprawidłowe dane.' : 'Request refused or invalid input.');
  expect(await query(
    `SELECT (SELECT count(*)::int FROM account_email_changes WHERE account_id=$1) AS changes,
            (SELECT count(*)::int FROM account_email_change_mail
             WHERE recipient=ANY($2::text[])) AS mail`,
    [account.id, [account.email, nextEmail, mismatchedEmail]],
  )).toEqual([{ changes: 0, mail: 0 }]);

  await page.setViewportSize(viewport);
  await submit(page, 'main form', changeInput);
  const [change] = await query('SELECT id FROM account_email_changes WHERE account_id=$1', [account.id]);
  expect(Boolean(change)).toBe(true);
  const [poison] = await query(
    `INSERT INTO account_email_change_mail(change_id,recipient,subject,body,created_at)
     VALUES ($1,'poison@example.invalid','Invalid fixture','missing link',now()-interval '1 day') RETURNING id`,
    [change.id],
  );
  const poisonState = () => query(
    `SELECT delivered_at IS NULL AS unsent, body='' AS erased, next_attempt_at='infinity' AS quarantined,
            updated_at FROM account_email_change_mail WHERE id=$1`,
    [poison.id],
  );

  // Bind a genuine STARTTLS endpoint and shut it down before dispatch.
  const down = catcher(jobs.env, (_message, callback) => callback(new Error('Unexpected delivery')));
  const port = await listen(down, 0);
  await close(down);
  const smtp = { OTERYN_SMTP_PORT: String(port) };
  await jobs.parkRecurring('identity.email-change', 60);
  const id = await enqueue('identity.email-change');
  const failed = jobs.start(smtp);
  await failed.ready;
  await waitForJob(id, 'done', 1); // SMTP retries are per message, not per sweep.
  failed.child.kill('SIGTERM');
  expect(await failed.exited).toEqual([0, null]);
  const [quarantine] = await poisonState();
  expect(quarantine).toMatchObject({ unsent: true, erased: true, quarantined: true });
  expect(await query(
    `SELECT delivery_attempts,delivered_at IS NULL AS unsent,
            extract(epoch FROM next_attempt_at-updated_at) BETWEEN 29 AND 31 AS backed_off
     FROM account_email_change_mail WHERE change_id=$1 AND id<>$2 ORDER BY recipient`,
    [change.id, poison.id],
  )).toEqual(Array.from({ length: 2 }, () => ({ delivery_attempts: 1, unsent: true, backed_off: true })));

  const accepted = [];
  let heldMail;
  let releaseMail;
  const up = catcher(jobs.env, (message, callback) => {
    const body = message.raw.replace(/=\r?\n/g, '');
    // Recovery-key enrollment also enqueues a security notification.
    if (!body.includes('/email-change/')) {
      callback();
      return;
    }
    const accept = () => {
      accepted.push(message);
      callback();
    };
    if (heldMail) {
      accept();
    } else {
      heldMail = message;
      releaseMail = accept;
    }
  });
  await listen(up, port);
  try {
    // Follow the actual PostgreSQL deadline with the same time source as the worker.
    await expect.poll(async () => (await query(
      `SELECT count(*)::int AS due FROM account_email_change_mail
       WHERE change_id=$1 AND delivered_at IS NULL AND next_attempt_at<=clock_timestamp()`,
      [change.id],
    ))[0].due, { timeout: 40_000 }).toBe(2);
    const retry = await enqueue('identity.email-change');
    const first = jobs.start(smtp);
    await first.ready;
    await expect.poll(() => Boolean(releaseMail)).toBe(true);
    expect(heldMail.to).toHaveLength(1);
    const [heldRecipient] = heldMail.to;
    const recipients = [account.email, nextEmail].sort();
    expect(recipients).toContain(heldRecipient);
    expect(accepted).toHaveLength(0);
    expect(await status(retry)).toMatchObject({ state: 'running', attempts: 1, released: false });

    // The first batch is already in SMTP DATA. Only the second process can
    // complete a new sweep while that batch still holds the first message.
    const overlap = await enqueue('identity.email-change');
    const second = jobs.start(smtp);
    await second.ready;
    await waitForJob(overlap, 'done', 1);
    expect(accepted.flatMap(message => message.to)).toEqual(recipients.filter(recipient => recipient !== heldRecipient));
    expect(await query(
      `SELECT recipient, delivered_at IS NOT NULL AS delivered, delivery_attempts
       FROM account_email_change_mail WHERE change_id=$1 AND id<>$2 ORDER BY recipient`,
      [change.id, poison.id],
    )).toEqual(recipients.map(recipient => ({
      recipient, delivered: recipient !== heldRecipient, delivery_attempts: 1,
    })));
    expect(await status(retry)).toMatchObject({ state: 'running', attempts: 1, released: false });

    const workers = [first, second];
    for (const worker of workers) expect(worker.child.kill('SIGTERM')).toBe(true);
    releaseMail();
    releaseMail = undefined;
    for (const worker of workers) expect(await worker.exited).toEqual([0, null]);
    await waitForJob(retry, 'done', 1);
    await waitForJob(overlap, 'done', 1);
    expect(accepted.flatMap(message => message.to).sort()).toEqual(recipients);
    expect(await query(
      `SELECT recipient, delivered_at IS NOT NULL AS delivered, body='' AS erased, delivery_attempts,
              delivered_at BETWEEN next_attempt_at AND updated_at AS delivered_after_backoff
       FROM account_email_change_mail WHERE change_id=$1 AND id<>$2 ORDER BY recipient`,
      [change.id, poison.id],
    )).toEqual(recipients.map(recipient => ({
      recipient, delivered: true, erased: true, delivery_attempts: 1, delivered_after_backoff: true,
    })));
    expect(await poisonState()).toEqual([quarantine]);

    const mail = accepted.find(message => message.to.includes(nextEmail));
    const raw = mail.raw.replace(/=\r?\n/g, '').replaceAll('=3D', '=');
    const link = raw.match(/https:\/\/localhost:\d+\/email-change\/confirm\/[a-f0-9]{64}/)?.[0];
    expect(Boolean(link)).toBe(true);
    expect(new URL(link).origin).toBe(jobs.env.OTERYN_PUBLIC_ORIGIN);
    // The confirmation link is a capability. Keep it out of assertion output.
    const response = await page.goto(link).catch(() => null);
    expect(response?.status()).toBe(200);
    await expect(page.locator('main form')).toBeVisible();
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    const missingCsrf = await page.request.post(link, {
      headers: { origin: jobs.env.OTERYN_PUBLIC_ORIGIN }, maxRedirects: 0,
    }).catch(() => null);
    expect(missingCsrf?.status()).toBe(403);
    await page.locator('main form button[type=submit]').click();
    await expect(page).toHaveURL(/\/login(?:\?|$)/);
    expect(await query('SELECT email FROM accounts WHERE id=$1', [account.id])).toEqual([{ email: nextEmail }]);
    await login(page, lang, { ...account, email: nextEmail });
    expect((await page.goto(`/${lang}/account/sessions`)).status()).toBe(200);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(lang === 'pl' ? 'Sesje web' : 'Web sessions');
  } finally {
    releaseMail?.();
    await close(up);
  }
});

test('two workers deliver one referral invitation and the recipient opens it', async ({ browser, page, jobs }, info) => {
  const lang = info.project.metadata.locale;
  const inviter = await seededPlayer(page, lang);
  const email = inviter.email.replace('outbox-', 'referral-');
  const delivered = [];
  let held = false;
  let releaseInvitation;
  const sink = catcher(jobs.env, (message, callback) => {
    if (!message.to.includes(email)) {
      callback();
      return;
    }
    const accept = () => {
      delivered.push(message);
      callback();
    };
    if (held) {
      accept();
    } else {
      held = true;
      releaseInvitation = accept;
    }
  });
  const port = await listen(sink, 0);
  const recipient = await browser.newContext({
    baseURL: process.env.PORTAL_E2E_ORIGIN, locale: lang, ignoreHTTPSErrors: true,
  });
  try {
    await query('UPDATE referral_campaign_settings SET enabled=true WHERE singleton');
    expect((await page.goto(`/${lang}/account/referrals`)).status()).toBe(200);
    await submit(page, 'form:has(#new-email)', { email });
    const [invitation] = await query(
      'SELECT id FROM referral_email_invitations WHERE inviter_id=$1 AND recipient=$2',
      [inviter.id, email],
    );
    expect(Boolean(invitation)).toBe(true);
    await jobs.parkRecurring('referrals.process', 60);
    const smtp = { OTERYN_SMTP_PORT: String(port) };
    const first = await enqueue('referrals.process');
    const firstWorker = jobs.start(smtp);
    await firstWorker.ready;
    await expect.poll(() => Boolean(releaseInvitation)).toBe(true);
    const replay = await enqueue('referrals.process');
    const replayWorker = jobs.start(smtp);
    await replayWorker.ready;
    // The replay may complete or wait on the first worker's delivery lock.
    // Its job is enqueued after the first worker reaches SMTP DATA.
    await expect.poll(async () => (await query(
      `SELECT state='done' OR state='running' AND EXISTS (
         SELECT FROM pg_stat_activity waiter
         JOIN pg_stat_activity holder ON holder.pid=ANY(pg_blocking_pids(waiter.pid))
         WHERE waiter.datname=current_database()
           AND waiter.application_name=$2 AND holder.application_name=$3
           AND waiter.wait_event='advisory' AND strpos(holder.query,'referral_email_attempts')>0
       ) AS ran FROM outbox WHERE id=$1 AND attempts=1`,
      [replay, replayWorker.name, firstWorker.name],
    ))[0]?.ran).toBe(true);
    expect(delivered).toHaveLength(0);
    expect(await status(first)).toMatchObject({ state: 'running', attempts: 1, released: false });
    releaseInvitation();
    releaseInvitation = undefined;
    await waitForJob(first, 'done', 1);
    await waitForJob(replay, 'done', 1);
    const workers = [firstWorker, replayWorker];
    for (const worker of workers) worker.child.kill('SIGTERM');
    for (const worker of workers) expect(await worker.exited).toEqual([0, null]);
    expect(await query(
      `SELECT delivery_attempts,delivered_at IS NOT NULL AS sent
       FROM referral_email_attempts WHERE invitation_id=$1`, [invitation.id],
    )).toEqual([{ delivery_attempts: 0, sent: true }]);
    expect(delivered.flatMap(message => message.to)).toEqual([email]);
    const url = `${jobs.env.OTERYN_PUBLIC_ORIGIN}/${lang}/referrals/${invitation.id}`;
    expect(delivered[0].raw.replace(/=\r?\n/g, '').includes(url)).toBe(true);
    const landing = await recipient.newPage();
    expect((await landing.goto(url)).status()).toBe(200);
    await expect(landing.getByRole('heading', { level: 1 })).toHaveText(lang === 'pl' ? 'Poleć znajomemu' : 'Tell a friend');
    await submit(landing);
    await expect(landing).toHaveURL(new RegExp(`/${lang}/register$`));
  } finally {
    releaseInvitation?.();
    await recipient.close();
    await close(sink);
    await query('UPDATE referral_campaign_settings SET enabled=false WHERE singleton');
  }
});
