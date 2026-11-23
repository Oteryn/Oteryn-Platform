import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, mailLink, submit } from '../helpers.mjs';
import { query, seededAccount, seededLogin, seededMfaAccount } from '../staff-rbac-helpers.mjs';

const root = '/admin/operational-mail';

async function grant(account, permissions) {
  const key = `operational_mail_${randomUUID().replaceAll('-', '')}`;
  const [{ id }] = await query('INSERT INTO roles(key) VALUES($1) RETURNING id', [key]);
  await query('INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)', [account.id, id]);
  await query('INSERT INTO role_permissions(role_id,permission) SELECT $1,unnest($2::text[])',
    [id, ['admin.access', ...permissions]]);
}

async function post(page, path, form, csrf) {
  return page.request.post(path, {
    form: { ...form, _csrf: csrf },
    headers: { origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });
}

test('i18n-parity for operational email', () => {
  const read = path => readFileSync(new URL(path, import.meta.url), 'utf8');
  const expectKeys = (source, prefix, required) => {
    const keys = [...source.matchAll(/^([a-z0-9-]+) = (\S.*)$/gm)]
      .map(([, key]) => key).filter(key => key.startsWith(prefix));
    expect(keys.sort()).toEqual([...required].sort());
  };
  const keys = (path, pattern) => [...read(path).matchAll(pattern)].map(([, key]) => key);
  const admin = new Set([
    ...keys('../../templates/admin/operational_mail.html', /tr\("(operational-mail-[a-z0-9-]+)"\)/g),
    ...keys('../../crates/admin/src/operational_mail/http.rs',
      /"(operational-mail-[a-z0-9-]+)"(?=\)|,\s*\))/g),
    ...keys('../../crates/admin/src/web.rs', /text\(lang, "(operational-mail-[a-z0-9-]+)"\)/g),
  ]);
  for (const audience of ['all', 'pl', 'en']) admin.add(`operational-mail-audience-${audience}`);
  for (const status of ['pending', 'approved', 'rejected', 'expired']) {
    admin.add(`operational-mail-status-${status}`);
  }
  for (const lang of ['pl', 'en']) {
    expectKeys(read(`../../templates/admin/${lang}.ftl`), 'operational-mail-', admin);
    expectKeys(read(`../../crates/notify/locales/${lang}.ftl`), 'operational-', ['operational-unsubscribe']);
    expectKeys(read(`../../crates/identity/src/account_lifecycle_${lang}.ftl`),
      'operational', ['operational', 'operational-opt-in']);
  }
});

test('a player opts in, staff preview and approve, and the player unsubscribes from the delivered mailing', async ({ page, browser }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const author = await seededMfaAccount(lang);
  const reviewer = await seededMfaAccount(lang);
  const recipient = await seededAccount(lang);
  const optedOut = await seededAccount(lang);
  await grant(author, ['mail.operational.compose', 'mail.operational.review']);
  await grant(reviewer, ['mail.operational.review']);

  const contexts = [];
  const open = async () => {
    const context = await browser.newContext({
      baseURL: new URL(page.url()).origin,
      ignoreHTTPSErrors: true,
      viewport: info.project.use.viewport,
      locale: lang,
    });
    contexts.push(context);
    return context.newPage();
  };
  const preferencePath = `/${lang}/account/notifications`;
  const setConsent = async (target, checked) => {
    await submit(target, 'main form', { operational_email: checked });
    await expect(target).toHaveURL(new RegExp(`${preferencePath}$`));
    await expect(target.locator('[name=operational_email]')).toBeChecked({ checked });
  };

  try {
    await seededLogin(page, lang, recipient);
    expect((await page.goto(preferencePath)).status()).toBe(200);
    await expect(page.locator('[name=operational_email]')).not.toBeChecked();
    await setConsent(page, true);

    const declined = await open();
    await seededLogin(declined, lang, optedOut);
    await declined.goto(preferencePath);
    await setConsent(declined, true);
    await setConsent(declined, false);
    expect((await declined.request.get(root, { maxRedirects: 0 })).status()).toBe(403);

    const composer = await open();
    await seededLogin(composer, lang, author);
    await composer.goto('/admin');
    await composer.locator(`nav a[href="${root}"]`).click();
    await expect(composer.locator('html')).toHaveAttribute('lang', lang);
    await expect(composer.locator('.op-shell__page main')).toBeVisible();
    expect(await composer.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    const form = composer.locator(`form[action="${root}"]`);
    const request_id = await form.locator('[name=request_id]').inputValue();
    const csrf = await form.locator('[name=_csrf]').inputValue();
    const marker = randomUUID();
    const draft = {
      request_id,
      audience: lang,
      subject_pl: 'ą'.repeat(200),
      body_pl: `Przerwa techniczna ${marker} <serwis>`,
      subject_en: 'W'.repeat(200),
      body_en: `Scheduled maintenance ${marker} <service>`,
    };
    await csrfRejected(composer, root, draft);
    expect((await post(composer, root, { ...draft, audience: 'unknown' }, csrf)).status()).toBe(400);
    await form.locator('[name=audience]').selectOption(lang);
    for (const field of ['subject_pl', 'body_pl', 'subject_en', 'body_en']) {
      await form.locator(`[name=${field}]`).fill(draft[field]);
    }
    await form.locator('button').click();
    const path = `${root}/${request_id}`;
    await expect(composer).toHaveURL(new RegExp(`${path}$`));
    await expect(composer.locator('article')).toHaveCount(1);
    await expect(composer.locator('article')).toContainText(draft[`body_${lang}`]);
    await expect(composer.locator('[data-campaign-status]')).toHaveAttribute('data-campaign-status', 'pending');
    expect(await composer.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    const selected = await query(
      'SELECT account_id FROM operational_mail_recipients WHERE campaign_id=$1 AND account_id=ANY($2::uuid[])',
      [request_id, [recipient.id, optedOut.id]],
    );
    expect(selected.map(row => row.account_id)).toEqual([recipient.id]);
    expect((await post(composer, path, { decision: 'approve' }, csrf)).status()).toBe(403);

    const approver = await open();
    await seededLogin(approver, lang, reviewer);
    await approver.goto(root);
    expect(await approver.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    await approver.locator(`main a[href="${path}"]`).click();
    const review = approver.locator(`form[action="${path}"]`);
    await csrfRejected(approver, path, { decision: 'approve' });
    const reviewCsrf = await review.locator('[name=_csrf]').inputValue();
    await review.locator('[value=approve]').click();
    await expect(approver.locator('[data-campaign-status]')).toHaveAttribute('data-campaign-status', 'approved');
    expect((await post(approver, path, { decision: 'approve' }, reviewCsrf)).status()).toBe(409);
    const audit = await query('SELECT action,actor FROM audit_log WHERE target=$1 ORDER BY created_at', [request_id]);
    expect(audit).toEqual([
      { action: 'mail.operational.previewed', actor: author.id },
      { action: 'mail.operational.approved', actor: reviewer.id },
    ]);

    const pattern = new RegExp(`https://[^\\s<>"]+/${lang}/unsubscribe\\?token[^\\s<>"]+`);
    const link = await mailLink(page, recipient.email, pattern, 90_000);
    const messages = await (await page.request.get('/__mail')).json();
    const matching = messages.filter(mail => mail.raw.replace(/=\r?\n/g, '').includes(marker));
    expect(matching.filter(mail => mail.to.includes(recipient.email))).toHaveLength(1);
    expect(matching.some(mail => mail.to.includes(optedOut.email))).toBe(false);
    await expect.poll(async () => {
      await approver.reload();
      return Number(await approver.locator('[data-count=delivered]').textContent());
    }).toBeGreaterThanOrEqual(1);

    expect((await page.goto(link)).status()).toBe(200);
    expect((await query(
      "SELECT email_enabled FROM account_notification_prefs WHERE account_id=$1 AND category='operational'",
      [recipient.id],
    ))[0].email_enabled).toBe(true);
    const origin = new URL(page.url()).origin;
    expect((await page.request.post(link, {
      form: { 'List-Unsubscribe': 'One-Click' }, headers: { origin },
    })).status()).toBe(403);
    await page.locator('main form button').click();
    expect((await page.goto(preferencePath)).status()).toBe(200);
    await expect(page.locator('[name=operational_email]')).not.toBeChecked();

    await setConsent(page, true);
    const mailClient = (await open()).request;
    expect((await mailClient.post(link, {
      form: { 'List-Unsubscribe': 'Unknown' },
    })).status()).toBe(422);
    const encoding = lang === 'pl' ? 'multipart' : 'form';
    expect((await mailClient.post(link, {
      [encoding]: { 'List-Unsubscribe': 'One-Click' },
    })).status()).toBe(200);
    await page.reload();
    await expect(page.locator('[name=operational_email]')).not.toBeChecked();
  } finally {
    await query(
      "UPDATE account_notification_prefs SET email_enabled=false WHERE account_id=ANY($1::uuid[]) AND category='operational'",
      [[recipient.id, optedOut.id]],
    );
    await Promise.all(contexts.map(context => context.close()));
  }
});


test('preview requests have a bounded allowance', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const author = await seededMfaAccount(lang);
  await grant(author, ['mail.operational.compose']);
  await seededLogin(page, lang, author);
  expect((await page.goto(root)).status()).toBe(200);
  const form = page.locator(`form[action="${root}"]`);
  const request_id = await form.locator('[name=request_id]').inputValue();
  const csrf = await form.locator('[name=_csrf]').inputValue();
  await query(`
    INSERT INTO web_rate_limits(limiter_name,key_hash,window_start,count)
    SELECT 'operational-mail-preview',sha256(uuid_send($1)),date_trunc('minute',now()) + window_offset,4
    FROM (VALUES (interval '0 minutes'), (interval '1 minute')) AS windows(window_offset)
  `, [author.id]);
  const draft = {
    request_id,
    audience: 'all',
    subject_pl: 'Przerwa',
    body_pl: '   ',
    subject_en: 'Maintenance',
    body_en: 'Scheduled maintenance',
  };
  expect((await post(page, root, draft, csrf)).status()).toBe(400);
  const limited = await post(page, root, draft, csrf);
  expect(limited.status()).toBe(429);
  expect(Number(limited.headers()['retry-after'])).toBeGreaterThan(0);
});
