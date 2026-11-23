import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected } from '../helpers.mjs';
import { query, seededMfaAccount, seededLogin, seededStaff } from '../staff-rbac-helpers.mjs';

test('staff panel links to the outbox', async ({ page }, info) => {
  await seededStaff(page, info.project.metadata.locale, 'platform_admin');
  await page.goto('/admin');
  await expect(page.locator('nav a[href="/admin/mail-outbox"]')).toBeVisible();
});

test('i18n-parity for the staff mail outbox', () => {
  const keys = lang => [...readFileSync(new URL(`../../templates/admin/${lang}.ftl`, import.meta.url), 'utf8')
    .matchAll(/^(mail-[a-z0-9-]+) = (.+)$/gm)].map(([, key]) => key).sort();
  const english = keys('en');
  expect(english.length).toBeGreaterThan(0);
  expect(new Set(english).size).toBe(english.length);
  expect(keys('pl')).toEqual(english);
});

test('staff find and manage mail jobs from the panel', async ({ page }, info) => {
  test.setTimeout(120_000);
  const lang = info.project.metadata.locale;
  const account = await seededMfaAccount(lang);
  const [{ id: role }] = await query('INSERT INTO roles(key) VALUES($1) RETURNING id', [`mail_ops_${randomUUID().replaceAll('-', '')}`]);
  await query('INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)', [account.id, role]);
  await query('INSERT INTO role_permissions(role_id,permission) SELECT $1,unnest($2::text[])', [
    role, ['admin.access', 'mail.outbox.view', 'mail.outbox.retry', 'mail.outbox.retry_batch'],
  ]);
  await seededLogin(page, lang, account);

  const recipient = account.email;
  const body = 'mail-body-fixture';
  const addJob = async (kind, state, error, age = '0 hours', address = recipient) => {
    const [{ id }] = await query(
      `INSERT INTO outbox(kind,payload,idempotency_key,state,attempts,last_error,run_after,created_at)
       VALUES($1,jsonb_build_object('email',$2::text,'fixture',$3::text,'account_id',$9::uuid,'locale',$10::text),$4,$5,$6,$7,now()+interval '1 day',now()-$8::interval)
       RETURNING id`,
      [kind, address, body, randomUUID(), state, error ? 1 : 0, error, age, account.id, lang],
    );
    return id;
  };
  const failed = await addJob('email.account_exists', 'pending', 'verification delivery failed');
  const failedBatch = await addJob('email.account_exists', 'pending', 'verification delivery failed', '2 days');
  const pending = await addJob('email.account_exists', 'pending', null);
  const sent = await addJob('email.account_exists', 'done', null);
  const dead = await addJob('email.account_exists', 'dead', 'verification delivery failed');
  const otherRecipient = await addJob('email.account_exists', 'done', null, '0 hours', `other-${recipient}`);

  const [{ id: verification }] = await query(
    "INSERT INTO outbox(kind,payload,idempotency_key,state) VALUES('email.verify',jsonb_build_object('token',$1::text),$2,'done') RETURNING id",
    [body, randomUUID()],
  );

  const path = '/admin/mail-outbox';
  const job = id => page.locator(`[data-job-id="${id}"]`);
  await page.goto('/admin');
  await page.locator(`nav a[href="${path}"]`).click();
  await expect(page.locator('h1')).toHaveText(lang === 'pl' ? 'Kolejka poczty' : 'Mail outbox');
  expect(await page.locator('html').getAttribute('lang')).toBe(lang);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);

  await expect(page.locator('.mail-counts')).toHaveCSS('display', 'grid');
  await expect(job(verification)).toBeVisible();
  await expect(job(verification)).not.toContainText(body);
  const filter = page.locator('form[method=get]');
  await filter.locator('[name=kind]').selectOption('email.account_exists');
  await filter.locator('[name=recipient]').fill(recipient);
  await filter.locator('button').click();
  for (const [id, state] of [[failed, 'failed'], [pending, 'pending'], [sent, 'sent'], [dead, 'dead']]) {
    await expect(job(id)).toHaveAttribute('data-state', state);
    await expect(job(id)).not.toContainText(recipient);
  }
  await expect(job(otherRecipient)).toHaveCount(0);
  await expect(page.locator('main')).not.toContainText(body);
  await expect(job(failed)).toContainText(lang === 'pl' ? 'Wysyłka wiadomości nie powiodła się' : 'Mail delivery failed');
  await expect(page.locator('[data-count="failed-7d"]')).toBeVisible();

  await filter.locator('[name=state]').selectOption('failed');
  const today = new Date().toISOString().slice(0, 10);
  await filter.locator('[name=from]').fill(today);
  await filter.locator('[name=to]').fill(today);
  await filter.locator('button').click();
  await expect(job(failed)).toBeVisible();
  await expect(job(failedBatch)).toHaveCount(0);
  await expect(job(sent)).toHaveCount(0);

  const mailCount = async () => {
    const response = await page.request.get('/__mail');
    expect(response.ok()).toBeTruthy();
    return (await response.json()).filter(message => message.to.includes(recipient)).length;
  };
  const initialMailCount = await mailCount();
  const retry = job(failed).locator('form[action$="/retry"]');
  await csrfRejected(page, `${path}/retry`, { id: failed });
  const csrf = await retry.locator('[name=_csrf]').inputValue();
  const post = (action, fields) => page.request.post(`${path}/${action}`, {
    form: { ...fields, _csrf: csrf },
    headers: { origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });
  expect((await post('cancel', { id: pending })).status()).toBe(403);
  expect((await post('retry', { id: dead })).status()).toBe(409);

  await retry.locator('button').click();
  await expect.poll(async () => (await query('SELECT action FROM audit_log WHERE actor=$1 AND target=$2', [account.id, failed])).map(row => row.action)).toContain('mail.outbox.retried');
  await expect.poll(async () => (await query('SELECT state FROM outbox WHERE id=$1', [failed]))[0].state).toBe('done');
  await expect.poll(mailCount).toBe(initialMailCount + 1);

  const batch = page.locator('form[action$="/retry-kind"]');
  await batch.locator('[name=kind]').selectOption('email.account_exists');
  await batch.locator('button').click();
  await expect.poll(async () => (await query('SELECT action FROM audit_log WHERE actor=$1 AND target=$2', [account.id, failedBatch])).map(row => row.action)).toContain('mail.outbox.batch_retried');

  await expect.poll(mailCount).toBe(initialMailCount + 2);

  await query('INSERT INTO role_permissions(role_id,permission) SELECT $1,unnest($2::text[])', [
    role, ['mail.outbox.cancel', 'mail.outbox.recipient'],
  ]);
  await page.goto(`${path}?${new URLSearchParams({ recipient })}`);
  await expect(job(pending)).toContainText(recipient);
  await job(pending).locator('form[action$="/cancel"] button').click();
  await expect(job(pending)).toHaveAttribute('data-state', 'cancelled');
  expect(await query("SELECT action FROM audit_log WHERE actor=$1 AND target=$2 AND action='mail.outbox.cancelled'", [account.id, pending])).toHaveLength(1);
});
