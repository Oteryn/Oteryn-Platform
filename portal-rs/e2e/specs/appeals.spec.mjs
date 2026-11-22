import { test, expect } from '../fixtures.mjs';
import { submit } from '../helpers.mjs';
import { action, audit, isolatedAdmin, player, query, unique } from '../staff-rbac-helpers.mjs';
import https from 'node:https';

function atTextareaLimit(prefix) {
  const firstLine = `${prefix}\n`;
  return firstLine + 'Details provided. '.repeat(250).slice(0, 3999 - firstLine.length) + 'x';
}

async function unfinishedAppeal(page, csrfHeader) {
  const target = new URL(page.url());
  const cookies = await page.context().cookies(target.href);
  const headers = {
    cookie: cookies.map(({ name, value }) => `${name}=${value}`).join('; '),
    origin: target.origin,
    'content-type': 'application/x-www-form-urlencoded',
    'transfer-encoding': 'chunked',
  };
  if (csrfHeader) headers['x-csrf-token'] = csrfHeader;
  return new Promise((resolve, reject) => {
    const request = https.request(target, { method: 'POST', headers, rejectUnauthorized: false }, response => {
      let body = '';
      response.setEncoding('utf8');
      response.on('error', reject);
      response.on('data', chunk => { body += chunk; });
      response.on('end', () => {
        resolve({ status: response.statusCode, cacheControl: response.headers['cache-control'], body });
        request.destroy();
      });
    });
    request.on('error', reject);
    request.setTimeout(4500, () => request.destroy(new Error('Appeal deadline did not close the unfinished upload')));
    // Keep the transport open after sending a partial form body.
    request.write('version=1&message=Incomplete');
  });
}

test('players appeal their own sanctions and staff uphold or overturn them', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const index = `/${lang}/support/appeals`;
  expect((await page.goto(index)).status()).toBe(403);
  const owner = await player(page, lang);
  const staff = await isolatedAdmin(browser, info);
  try {
    for (const [decision, sanctionStatus, label] of [
      ['accepted', 'revoked', lang === 'pl' ? 'Uwzględnione' : 'Overturned'],
      ['rejected', 'active', lang === 'pl' ? 'Odrzucone' : 'Upheld'],
    ]) {
      const reason = unique();
      const privateNote = unique();
      const submittedReason = `Please review <${unique()}>`;
      const reviewedReason = `Reviewed ${unique()}`;
      const message = decision === 'accepted' ? atTextareaLimit(submittedReason) : submittedReason;
      const outcome = decision === 'accepted' ? atTextareaLimit(reviewedReason) : reviewedReason;
      if (decision === 'accepted') {
        expect(message).toHaveLength(4000);
        expect(outcome).toHaveLength(4000);
      }
      expect((await staff.page.goto('/admin/moderation/enforcement/create')).status()).toBe(200);
      const now = Math.floor(Date.now() / 1000);
      await action(staff.page, 'form:has(input[value=sanction])', {
        target: owner.id,
        category: 'restriction',
        status: 'active',
        public_reason: reason,
        moderator_notes: privateNote,
        effective_at: String(now - 1),
        expires_at: String(now + 3600),
      });
      await expect(staff.page).toHaveURL(/enforcement\/[a-f0-9-]+$/);
      const staffPath = new URL(staff.page.url()).pathname;
      const id = staffPath.split('/').at(-1);
      const playerPath = `${index}/${id}`;

      expect((await staff.page.request.get(playerPath)).status()).toBe(404);
      expect((await page.request.get(staffPath)).status()).toBe(403);
      expect((await page.goto(`/${lang}/`)).status()).toBe(200);
      const mobileMenu = page.locator('#portal-menu-toggle');
      if (await mobileMenu.isVisible()) await mobileMenu.click();
      await page.locator('[data-account-link]').click();
      const appealsLink = page.locator('#podglad-konta').getByRole('link', {
        name: lang === 'pl' ? 'Odwołania od sankcji' : 'Sanction appeals',
        exact: true,
      });
      await expect(appealsLink).toHaveAttribute('href', index);
      const [list] = await Promise.all([
        page.waitForResponse(response => new URL(response.url()).pathname === index),
        appealsLink.click(),
      ]);
      await expect(page).toHaveURL(index);
      expect(list.status()).toBe(200);
      expect(list.headers()['cache-control']).toContain('no-store');
      expect(list.headers()['content-security-policy']).toContain("frame-ancestors 'none'");
      await expect(page.locator('html')).toHaveAttribute('lang', lang);
      await page.getByRole('link', { name: reason, exact: true }).click();
      await expect(page).toHaveURL(playerPath);
      await expect(page.locator('main')).not.toContainText(privateNote);
      if (decision === 'accepted') {
        const fields = await page.locator('main form').evaluate(form => Object.fromEntries(new FormData(form)));
        for (const csrfHeader of [undefined, fields._csrf]) {
          const unfinished = await unfinishedAppeal(page, csrfHeader);
          expect(unfinished.status).toBe(503);
          expect(unfinished.cacheControl).toContain('no-store');
          expect(unfinished.body).toContain(lang === 'pl' ? 'Usługa chwilowo niedostępna' : 'Temporarily unavailable');
        }
        const staleCsrf = await page.request.post(playerPath, {
          form: { ...fields, _csrf: '0'.repeat(64), message },
          headers: { origin: new URL(page.url()).origin },
          maxRedirects: 0,
        });
        expect(staleCsrf.status()).toBe(403);
        expect(await staleCsrf.text()).toContain(lang === 'pl' ? 'Brak dostępu' : 'Access denied');
        const invalid = await page.request.post(playerPath, {
          form: { ...fields, message: '   ' },
          headers: { origin: new URL(page.url()).origin },
          maxRedirects: 0,
        });
        expect(invalid.status()).toBe(422);
        expect(await invalid.text()).toContain(lang === 'pl' ? 'Sprawdź wprowadzone dane' : 'Please check your input');
      }
      await submit(page, 'main form', { message });
      await expect(page.locator('main')).toContainText(message);
      await expect(page.locator('main')).toContainText(lang === 'pl' ? 'Oczekuje na rozpatrzenie' : 'Awaiting review');

      expect((await staff.page.goto("/admin/moderation/enforcement")).status()).toBe(200);
      await expect(staff.page.getByRole("link", { name: reason, exact: true })).toHaveAttribute("href", staffPath);
      expect((await staff.page.goto(staffPath)).status()).toBe(200);
      await expect(staff.page.locator('main')).toContainText(message);
      const form = staff.page.locator('form:has(input[value=appeal])');
      const selector = form.locator('[name=appeal_status]');
      const outcomeField = form.locator('[name=appeal_outcome]');
      await expect(outcomeField).toHaveJSProperty('required', true);
      await expect(outcomeField).toHaveJSProperty('maxLength', 4000);
      await expect(selector).toHaveValue('');
      await expect(selector.locator('option:checked')).toHaveJSProperty('disabled', true);
      await outcomeField.fill(outcome);
      await form.locator('button[type=submit]').click();
      expect(await selector.evaluate(field => field.validity.valueMissing)).toBe(true);
      expect(await query(
        'SELECT action FROM audit_log WHERE action=$1 AND target=$2',
        ['support.enforcement_appeal_updated', id],
      )).toHaveLength(0);
      const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
      await action(staff.page, 'form:has(input[value=appeal])', { appeal_status: decision, appeal_outcome: outcome });
      await expect(staff.page.locator('main')).toContainText(outcome);
      await expect(form).toHaveCount(0);
      expect(await query('SELECT message, outcome FROM support_appeal_workflows WHERE sanction_id=$1', [id]))
        .toEqual([{ message, outcome }]);

      const repeated = await staff.page.request.post(staffPath, {
        form: { ...fields, appeal_status: decision, appeal_outcome: outcome },
        headers: { origin: new URL(staff.page.url()).origin, 'accept-language': lang },
        maxRedirects: 0,
      });
      expect(repeated.status()).toBe(409);
      expect(await repeated.text()).toContain(lang === 'pl' ? 'Ta strona uległa zmianie' : 'This page has changed');
      const decisionLabel = decision === 'accepted'
        ? (lang === 'pl' ? 'Odwołanie uwzględnione' : 'Appeal accepted')
        : (lang === 'pl' ? 'Odwołanie odrzucone' : 'Appeal rejected');
      const notices = await query("SELECT body FROM notifications WHERE account_id=$1 AND category='support'", [owner.id]);
      expect(notices.some(notice => notice.body.includes(`${decisionLabel}: ${outcome}`))).toBe(true);
      expect((await query('SELECT status FROM sanctions WHERE id=$1', [id]))[0].status).toBe(sanctionStatus);
      await page.reload();
      await expect(page.locator('main')).toContainText(label);
      await expect(page.locator('main')).toContainText(outcome);
      await expect(page.locator('main')).not.toContainText(privateNote);
      if (decision === 'rejected') {
        await expect(page.locator('main form')).toHaveCount(1);
        await expect(page.locator('main form [name=message]')).toBeVisible();
      } else {
        await expect(page.locator('main form')).toHaveCount(0);
      }
      await audit(staff.page, 'support.enforcement_appeal_updated', id);
      expect(await query(
        'SELECT actor FROM audit_log WHERE action=$1 AND target=$2',
        ['support.enforcement_appeal_submitted', id],
      )).toEqual([{ actor: owner.id }]);
      expect(await query(
        'SELECT action FROM audit_log WHERE action=$1 AND target=$2',
        ['support.enforcement_appeal_updated', id],
      )).toHaveLength(1);
      if (decision === 'rejected') {
        await expect.poll(async () => Number((await query(
          "SELECT coalesce(max(count),0) AS count FROM web_rate_limits WHERE limiter_name='support-enforcement-appeal' AND window_start=date_trunc('minute', clock_timestamp())",
        ))[0].count), { timeout: 65_000 }).toBeLessThan(3);
        const newEvidence = `New evidence ${unique()}`;
        await submit(page, 'main form', { message: newEvidence });
        await expect(page.locator('main')).toContainText(newEvidence);
        await expect(page.locator('main')).not.toContainText(outcome);
        await expect(page.locator('main form')).toHaveCount(0);
        expect(await query(
          "SELECT before->>'status' AS status FROM audit_log WHERE action=$1 AND target=$2 ORDER BY created_at, id",
          ['support.enforcement_appeal_submitted', id],
        )).toEqual([{ status: 'none' }, { status: 'rejected' }]);
      }
    }
    const detail = new URL(page.url()).pathname;
    const playerAdmin = `/admin/players/${owner.id}`;
    expect((await staff.page.goto(playerAdmin)).status()).toBe(200);
    await action(staff.page, 'form:has(button[value=kick])', { reason: 'Close game sessions' }, 'kick');
    await audit(staff.page, 'players.kick', owner.id);
    for (const path of [index, detail]) {
      const staleSession = await page.goto(path);
      expect(staleSession.status()).toBe(403);
      await expect(page.locator('main')).toContainText(
        lang === 'pl' ? 'Zaloguj się ponownie, aby skorzystać z odwołań.' : 'Sign in again to use appeals.',
      );
      await expect(page.locator('main form')).toHaveCount(0);
    }
  } finally {
    await staff.context.close();
  }
});
