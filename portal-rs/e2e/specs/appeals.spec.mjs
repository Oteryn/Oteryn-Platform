import { test, expect } from '../fixtures.mjs';
import { pool, submit } from '../helpers.mjs';
import { action, audit, isolatedAdmin, player, query, unique } from '../staff-rbac-helpers.mjs';
import { readFileSync } from 'node:fs';
import https from 'node:https';

function atTextareaLimit(prefix) {
  const firstLine = `${prefix}\n`;
  return firstLine + 'Details provided. '.repeat(250).slice(0, 3999 - firstLine.length) + 'x';
}

async function unfinishedAppeal(page, csrfHeader) {
  const target = new URL(page.url());
  const caFile = process.env.SSL_CERT_FILE;
  if (!caFile) throw new Error('The browser harness must supply its TLS CA in SSL_CERT_FILE');
  const ca = readFileSync(caFile);
  const cookies = await page.context().cookies(target.href);
  const headers = {
    cookie: cookies.map(({ name, value }) => `${name}=${value}`).join('; '),
    origin: target.origin,
    'content-type': 'application/x-www-form-urlencoded',
    'transfer-encoding': 'chunked',
  };
  if (csrfHeader) headers['x-csrf-token'] = csrfHeader;
  return new Promise((resolve, reject) => {
    const request = https.request(target, { method: 'POST', headers, ca }, response => {
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

async function postWithLock(page, path, form, sql, params = []) {
  const db = pool();
  const lock = await db.connect();
  try {
    await lock.query('BEGIN');
    const locked = await lock.query(sql, params);
    if (locked.command === 'SELECT') expect(locked.rowCount).toBeGreaterThan(0);
    return await page.request.post(path, {
      form,
      headers: { origin: new URL(page.url()).origin },
      maxRedirects: 0,
      timeout: 4500,
    });
  } finally {
    await lock.query('ROLLBACK');
    lock.release();
    await db.end();
  }
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
        // Block each shared chrome lookup independently.
        for (const lock of [
          'LOCK TABLE web_sessions IN ACCESS EXCLUSIVE MODE',
          'LOCK TABLE accounts IN ACCESS EXCLUSIVE MODE',
        ]) {
          const blocked = await postWithLock(page, playerPath, { ...fields, message }, lock);
          expect(blocked.status()).toBe(503);
          expect(blocked.headers()['cache-control']).toContain('no-store');
          expect(await blocked.text()).toContain(lang === 'pl' ? 'Usługa chwilowo niedostępna' : 'Temporarily unavailable');
        }
        const malformed = await page.request.post(`${index}/not-a-uuid`, {
          form: { ...fields, message },
          headers: { origin: new URL(page.url()).origin },
          maxRedirects: 0,
        });
        expect(malformed.status()).toBe(400);
        expect(malformed.headers()['content-type']).toContain('text/html');
        expect(await malformed.text()).toContain(lang === 'pl' ? 'Nieprawidłowe żądanie' : 'Invalid request');
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
      const staleTab = decision === 'rejected' ? await page.context().newPage() : null;
      if (staleTab) expect((await staleTab.goto(playerPath)).status()).toBe(200);
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
      if (decision === 'accepted') {
        const blocked = await postWithLock(staff.page, staffPath, {
          ...fields,
          appeal_status: decision,
          appeal_outcome: outcome,
        }, 'SELECT s.id_hash FROM web_sessions s JOIN accounts a ON a.id=s.account_id WHERE a.email=$1 FOR UPDATE OF s',
        [staff.account.email]);
        expect(blocked.status()).toBe(503);
        expect(blocked.headers()['cache-control']).toContain('no-store');
        expect(await query('SELECT status FROM support_appeal_workflows WHERE sanction_id=$1', [id]))
          .toEqual([{ status: 'requested' }]);

        await selector.selectOption(decision);
        await outcomeField.fill('   ');
        const [invalid] = await Promise.all([
          staff.page.waitForResponse(response => response.request().method() === 'POST'
            && new URL(response.url()).pathname === staffPath),
          form.locator('button[type=submit]').click(),
        ]);
        expect(invalid.status()).toBe(422);
        expect(invalid.headers()['cache-control']).toContain('no-store');
        await expect(staff.page.locator('main')).toContainText(
          lang === 'pl' ? 'Uzasadnienie nie może być puste' : 'enter a non-blank decision reason',
        );
        const edit = staff.page.getByRole('link', { name: lang === 'pl' ? 'Wróć do odwołania' : 'Back to appeal' });
        await expect(edit).toHaveAttribute('href', staffPath);
        await edit.click();
        await expect(form).toBeVisible();
        expect(await query(
          'SELECT action FROM audit_log WHERE action=$1 AND target=$2',
          ['support.enforcement_appeal_updated', id],
        )).toHaveLength(0);
      }
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
        ))[0].count), { timeout: 65_000 }).toBe(0);
        try {
          const form = staleTab.locator('main form');
          await form.locator('textarea[name=message]').fill('Submitted from an old tab');
          const [posted] = await Promise.all([
            staleTab.waitForResponse(response => response.request().method() === 'POST'
              && new URL(response.url()).pathname === playerPath),
            form.locator('button[type=submit]').click(),
          ]);
          expect(posted.status()).toBe(409);
          const current = staleTab.getByRole('link', { name: lang === 'pl' ? 'Otwórz odwołanie' : 'Open current appeal' });
          await expect(current).toHaveAttribute('href', playerPath);
          const loaded = staleTab.waitForResponse(response => response.request().method() === 'GET'
            && new URL(response.url()).pathname === playerPath);
          await current.click();
          expect((await loaded).status()).toBe(200);
          await expect(staleTab.locator('main')).toContainText(outcome);
        } finally {
          await staleTab.close();
        }
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
    const [{ id: expiring }] = await query(
      `INSERT INTO sanctions (account_id, category, status, public_reason, effective_at, expires_at, created_by)
       SELECT $1, 'restriction', 'active', $2, clock_timestamp() - interval '1 minute',
              clock_timestamp() + interval '1 hour', id FROM accounts WHERE email = $3 RETURNING id`,
      [owner.id, unique(), staff.account.email],
    );
    const expiryTab = await page.context().newPage();
    try {
      const expiryPath = `${index}/${expiring}`;
      expect((await expiryTab.goto(expiryPath)).status()).toBe(200);
      const form = expiryTab.locator('main form');
      await expect(form).toBeVisible();
      await query('UPDATE sanctions SET expires_at=clock_timestamp() WHERE id=$1', [expiring]);
      await form.locator('textarea').fill('Submitted after expiry');
      const [posted] = await Promise.all([
        expiryTab.waitForResponse(response => response.request().method() === 'POST'
          && new URL(response.url()).pathname === expiryPath),
        form.locator('button[type=submit]').click(),
      ]);
      expect(posted.status()).toBe(409);
      await expect(expiryTab.locator('main')).toContainText(
        lang === 'pl' ? 'Ta sankcja nie jest już aktywna.' : 'This sanction is no longer active.',
      );
      const appeals = expiryTab.getByRole('link', { name: lang === 'pl' ? 'Wróć do odwołań' : 'Back to appeals' });
      await expect(appeals).toHaveAttribute('href', index);
      const [recovered] = await Promise.all([
        expiryTab.waitForResponse(response => response.request().method() === 'GET'
          && new URL(response.url()).pathname === index),
        appeals.click(),
      ]);
      expect(recovered.status()).toBe(200);
      expect(await query('SELECT status FROM support_appeal_workflows WHERE sanction_id=$1', [expiring])).toEqual([]);
    } finally {
      await expiryTab.close();
    }
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
