import {randomBytes, randomUUID} from 'node:crypto';
import {test, expect} from '../../fixtures.mjs';
import {csrfRejected, mailLink, submit} from '../../helpers.mjs';
import {action, confirmStaffPassword, isolatedAdmin, player, query, rolePage, unique} from '../../staff-rbac-helpers.mjs';

const labels = {
  pl: ['Zapisz oznaczenie', 'Zapisz status konta', 'Zleć zmianę adresu e-mail'],
  en: ['Save review flag', 'Save account status', 'Request email change'],
};
const reason = 'support_case';
const actions = [
  ['note', 'Support reviewed this case', '   ', 'players.note.added'],
  ['review-flag', 'yes', 'unknown', 'players.flags.changed'],
  ['account-email', null, 'invalid-address', 'players.email.change_requested'],
  ['account-status', 'suspended', 'closed', 'players.status.changed'],
];

async function seedAccount(locale) {
  const email = `${unique()}@example.test`;
  const [account] = await query(
    "INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,'test-fixture','active',$2,now()) RETURNING id",
    [email, locale],
  );
  return {...account, email};
}

async function post(page, path, form) {
  return page.request.post(path, {
    form, headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
  });
}

test('staff account forms enforce input, permission, MFA session and CSRF', async ({page, browser, request}, info) => {
  test.setTimeout(420_000);
  const locale = info.project.metadata.locale;
  const target = await player(page, locale);
  const nextEmail = `${unique()}@example.test`;
  const path = `/admin/players/${target.id}`;
  const admin = await isolatedAdmin(browser, info);
  try {
    expect((await request.get(path, {maxRedirects: 0})).status()).toBe(303);
    expect((await page.request.get(path, {maxRedirects: 0})).status()).toBe(403);
    expect((await admin.page.goto(path)).status()).toBe(200);
    for (const label of labels[locale]) {
      await expect(admin.page.getByRole('button', {name: label})).toBeVisible();
    }
    expect(await admin.page.locator('html').evaluate(element =>
      element.scrollWidth <= element.clientWidth,
    )).toBe(true);

    for (const [operation, value, invalid, auditAction] of actions) {
      const selector = `form:has(button[value="${operation}"])`;
      const csrf = await admin.page.locator(selector).locator('input[name=_csrf]').inputValue();
      const body = value ?? nextEmail;
      const submission = operation === 'note'
        ? {submission: await admin.page.locator(selector).locator('[name=submission]').inputValue()}
        : {};
      await csrfRejected(admin.page, path, {action: operation, body, reason, ...submission});
      for (const fields of [{body: invalid, reason}, {body, reason: 'private@example.test'}]) {
        expect((await post(admin.page, path, {action: operation, _csrf: csrf, ...submission, ...fields})).status())
          .toBe(422);
      }
      const [unchanged] = await query(
        'SELECT count(*)::int AS count FROM audit_log WHERE target=$1 AND action=$2',
        [target.id, auditAction],
      );
      expect(unchanged.count).toBe(0);

      await action(admin.page, selector, {body, reason}, operation);
      await expect(admin.page).toHaveURL(new RegExp(`${path}$`));
      if (operation === 'note') {
        const retry = {action: operation, _csrf: csrf, body, reason, ...submission};
        expect((await post(admin.page, path, retry)).status()).toBe(303);
        expect((await post(admin.page, path, {...retry, body: 'Different note'})).status()).toBe(409);
        expect(await query('SELECT body FROM player_notes WHERE account_id=$1', [target.id]))
          .toEqual([{body}]);
      }
      const entries = await query(
        'SELECT before, after, staff_reason FROM audit_log WHERE target=$1 AND action=$2',
        [target.id, auditAction],
      );
      expect(entries).toHaveLength(1);
      expect(entries[0].staff_reason).toBe(reason);
      expect(JSON.stringify(entries)).not.toContain(target.email);
      if (operation === 'note' || operation === 'account-email') {
        expect(JSON.stringify(entries)).not.toContain(body);
      }

      if (operation === 'account-email') {
        const [proposal] = await query(
          `SELECT c.owner_approved_at IS NULL AS needs_owner, c.confirmed_at IS NULL AS unconfirmed,
                  m.recipient=c.old_email AS sent_to_owner
           FROM account_email_changes c JOIN account_email_change_mail m ON m.change_id=c.id
           WHERE c.account_id=$1`, [target.id],
        );
        expect(proposal).toEqual({needs_owner: true, unconfirmed: true, sent_to_owner: true});
        expect(await query('SELECT id FROM account_email_change_mail WHERE recipient=$1', [nextEmail])).toHaveLength(0);

        const settingsLink = await mailLink(
          page, target.email, new RegExp(`https://localhost:18443/${locale}/account/security/email`), 85_000,
        );
        await page.goto(`/${locale}/account/recovery-key`);
        await submit(page, 'main form', {current_password: target.password});
        const key = (await page.locator('.op-code').innerText()).trim();
        await submit(page, 'main form', {current_password: target.password, recovery_key: key});
        await page.goto(settingsLink);
        await submit(page, 'main form', {
          email: nextEmail, email_confirmation: nextEmail, current_password: target.password,
          proof: key, recovery: true,
        });
        const confirm = await mailLink(page, nextEmail, /https:\/\/localhost:18443\/email-change\/confirm\/[a-f0-9]{64}/, 85_000);
        await mailLink(page, target.email, /https:\/\/localhost:18443\/email-change\/revert\/[a-f0-9]{64}/, 85_000);
        await page.goto(confirm);
        await submit(page);
        const [pending] = await query(
          `SELECT a.email, c.confirmed_at IS NOT NULL AS confirmed, c.applied_at IS NULL AS waiting
           FROM accounts a JOIN account_email_changes c ON c.account_id=a.id
           WHERE a.id=$1 AND c.initiated_by_staff AND c.owner_approved_at IS NOT NULL`,
          [target.id],
        );
        expect(pending).toEqual({email: target.email, confirmed: true, waiting: true});
        // Advance the stored deadline in the disposable database. The real
        // worker has to apply the verified request and keep the address verified.
        await query("UPDATE account_email_changes SET apply_after=now()-interval '1 second' WHERE account_id=$1 AND owner_approved_at IS NOT NULL", [target.id]);
        await expect.poll(async () => {
          const [applied] = await query(
            'SELECT email=$2 AND email_verified_at IS NOT NULL AS verified FROM accounts WHERE id=$1',
            [target.id, nextEmail],
          );
          return applied.verified;
        }, {timeout: 85_000}).toBe(true);
      }
    }

    await expect(admin.page.locator('#review-flag')).toHaveValue('yes');
    await expect(admin.page.locator('#account-status')).toHaveValue('suspended');
    await expect(admin.page.locator('main')).toContainText(actions[0][1]);
    const [actor] = await query('SELECT id FROM accounts WHERE email=$1', [admin.account.email]);
    const csrfInput = 'form:has(button[value=account-status]) input[name=_csrf]';
    const expired = await admin.page.locator(csrfInput).inputValue();
    await query(
      'UPDATE web_sessions SET csrf_token=$3 WHERE account_id=$1 AND csrf_token=$2',
      [actor.id, expired, randomBytes(32).toString('hex')],
    );
    const stale = await admin.page.request.post(path, {
      form: {action: 'account-status', body: 'active', reason, _csrf: expired},
      headers: {origin: new URL(admin.page.url()).origin, 'accept-language': locale},
      maxRedirects: 0,
    });
    expect(stale.status()).toBe(403);
    const [title, explanation] = locale === 'pl'
      ? ['Brak dostępu', 'Nie masz uprawnień do wyświetlenia tej strony.']
      : ['Access denied', 'You do not have permission to view this page.'];
    const refusal = await stale.text();
    expect(refusal).toContain(`lang="${locale}"`);
    expect(refusal).toContain(title);
    expect(refusal).toContain(explanation);
    expect((await query('SELECT status FROM accounts WHERE id=$1', [target.id]))[0].status)
      .toBe('suspended');

    await admin.page.goto(path);
    const csrf = await admin.page.locator(csrfInput).inputValue();
    expect(csrf !== expired).toBe(true);
    const protectedAccount = await seedAccount(locale);
    await query(
      "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'",
      [protectedAccount.id],
    );
    const protectedPath = `/admin/players/${protectedAccount.id}`;
    expect((await admin.page.goto(protectedPath)).status()).toBe(200);
    await expect(admin.page.locator('button[value=review-flag],button[value=account-status],button[value=account-email]')).toHaveCount(0);
    for (const [operation, body] of [
      ['review-flag', 'yes'],
      ['review-flag', 'no'],
      ['account-status', 'suspended'],
      ['account-email', `${unique()}@example.test`],
    ]) {
      expect((await post(admin.page, protectedPath, {
        action: operation, body, reason, _csrf: csrf,
      })).status()).toBe(403);
    }
    expect(await query('SELECT status,email FROM accounts WHERE id=$1', [protectedAccount.id]))
      .toEqual([{status: 'active', email: protectedAccount.email}]);
    expect(await query('SELECT id FROM account_email_changes WHERE account_id=$1', [protectedAccount.id]))
      .toHaveLength(0);
    expect(await query('SELECT account_id FROM staff_account_flags WHERE account_id=$1', [protectedAccount.id]))
      .toHaveLength(0);

    await query("UPDATE account_roles SET role_id=(SELECT id FROM roles WHERE key='security_admin') WHERE account_id=$1", [actor.id]);
    expect((await admin.page.request.get(path, {maxRedirects: 0})).status()).toBe(403);
    expect((await post(admin.page, path, {
      action: 'note', body: 'Unauthorized note', reason, _csrf: csrf, submission: randomUUID(),
    })).status()).toBe(403);

    // Keep the real MFA session while reducing its durable role to support.
    await query("UPDATE account_roles SET role_id=(SELECT id FROM roles WHERE key='support') WHERE account_id=$1", [actor.id]);
    expect((await admin.page.goto(path)).status()).toBe(200);
    await expect(admin.page.locator('button[value=review-flag],button[value=account-status],button[value=account-email]')).toHaveCount(0);
    for (const [operation, value] of actions.slice(1)) {
      expect((await post(admin.page, path, {
        action: operation, body: value ?? nextEmail, reason, _csrf: csrf,
      })).status()).toBe(403);
    }
  } finally {
    await admin.context.close();
  }
});

test('role grants cannot delegate an unheld permission', async ({browser}, info) => {
  test.setTimeout(180_000);
  const locale = info.project.metadata.locale;
  const reason = 'role_assignment';
  const target = await seedAccount(locale);
  const admin = await isolatedAdmin(browser, info);
  try {
    const [actor] = await query('SELECT id FROM accounts WHERE email=$1', [admin.account.email]);
    const [role] = await query('INSERT INTO roles(key) VALUES($1) RETURNING id', [unique()]);
    await query("INSERT INTO role_permissions(role_id,permission) VALUES($1,'account.flags.edit')", [role.id]);
    await rolePage(admin.page, role.id);
    const selector = 'form:has(button[value=grant])';
    const form = {
      action: 'grant', account: target.id, role: role.id, reason,
      _csrf: await admin.page.locator(selector).locator('input[name=_csrf]').inputValue(),
    };
    await csrfRejected(admin.page, '/admin/roles', form);
    expect((await post(admin.page, '/admin/roles', {...form, reason: ''})).status()).toBe(422);
    expect((await post(admin.page, '/admin/roles', {...form, reason: 'private@example.test'})).status()).toBe(422);
    await query('UPDATE web_sessions SET sudo_until=clock_timestamp() WHERE account_id=$1', [actor.id]);
    const expired = await admin.page.request.post('/admin/roles', {
      form, headers: {origin: new URL(admin.page.url()).origin, 'accept-language': locale},
      maxRedirects: 0,
    });
    expect(expired.status()).toBe(403);
    expect(await expired.text()).toContain(locale === 'pl'
      ? 'Potwierdź hasło przed zmianą przypisania ról.'
      : 'Confirm your password before changing role assignments.');
    expect(await query('SELECT 1 FROM account_roles WHERE account_id=$1', [target.id])).toHaveLength(0);
    expect(await query("SELECT 1 FROM audit_log WHERE target=$1 AND action='rbac.role.granted'", [target.id])).toHaveLength(0);
    await confirmStaffPassword(admin.page, locale, admin.password);
    await rolePage(admin.page, role.id);
    form._csrf = await admin.page.locator(selector).locator('input[name=_csrf]').inputValue();
    const [manager] = await query('INSERT INTO roles(key) VALUES($1) RETURNING id', [unique()]);
    await query(
      "INSERT INTO role_permissions(role_id,permission) VALUES($1,'admin.access'),($1,'admin.roles.manage'),($1,'admin.roles.grant')",
      [manager.id],
    );
    await query("UPDATE account_roles SET role_id=$2 WHERE account_id=$1", [actor.id, manager.id]);
    expect((await post(admin.page, '/admin/roles', form)).status()).toBe(403);
    expect(await query('SELECT 1 FROM account_roles WHERE account_id=$1', [target.id])).toHaveLength(0);
    await query("INSERT INTO role_permissions(role_id,permission) VALUES($1,'account.flags.edit')", [manager.id]);
    await action(admin.page, selector, {account: target.id, role: role.id, reason}, 'grant');
    expect(await query('SELECT 1 FROM account_roles WHERE account_id=$1 AND role_id=$2', [target.id, role.id])).toHaveLength(1);
    expect(await query("SELECT 1 FROM audit_log WHERE target=$1 AND action='rbac.role.granted'", [target.id])).toHaveLength(1);
    const widened = {
      action: 'set-permissions', role: role.id, permissions: 'account.flags.edit admin.access',
      reason, _csrf: form._csrf,
    };
    await csrfRejected(admin.page, '/admin/roles', widened);
    await query("DELETE FROM role_permissions WHERE role_id=$1 AND permission='admin.roles.grant'", [manager.id]);
    expect((await post(admin.page, '/admin/roles', widened)).status()).toBe(403);
    await query("INSERT INTO role_permissions(role_id,permission) VALUES($1,'admin.roles.grant')", [manager.id]);
    expect((await post(admin.page, '/admin/roles', {...widened, reason: ''})).status()).toBe(422);
    expect(await query("SELECT 1 FROM role_permissions WHERE role_id=$1 AND permission='admin.access'", [role.id]))
      .toHaveLength(0);
    await rolePage(admin.page, role.id);
    await action(admin.page, 'form:has(button[value=set-permissions])', {
      role: role.id, permissions: widened.permissions, reason,
    }, 'set-permissions');
    expect(await query("SELECT staff_reason FROM audit_log WHERE target=$1 AND action='rbac.role.permissions_changed'", [role.id]))
      .toEqual([{staff_reason: reason}]);

  } finally {
    await admin.context.close();
  }
});
