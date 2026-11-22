import {test, expect} from '../../fixtures.mjs';
import {csrfRejected, mailLink, submit} from '../../helpers.mjs';
import {action, isolatedAdmin, query, rolePage, unique} from '../../staff-rbac-helpers.mjs';

const labels = {
  pl: ['Zapisz oznaczenie', 'Zapisz status konta', 'Zleć zmianę adresu e-mail'],
  en: ['Save review flag', 'Save account status', 'Request email change'],
};
const reason = 'Verified support case';
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

test('staff account forms enforce input, permission, MFA session and CSRF', async ({page, browser}, info) => {
  test.setTimeout(300_000);
  const locale = info.project.metadata.locale;
  const target = await seedAccount(locale);
  const nextEmail = `${unique()}@example.test`;
  const path = `/admin/players/${target.id}`;
  const admin = await isolatedAdmin(browser, info);
  try {
    expect((await page.request.get(path, {maxRedirects: 0})).status()).toBe(303);
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
      await csrfRejected(admin.page, path, {action: operation, body, reason});
      for (const fields of [{body: invalid, reason}, {body, reason: 'short'}]) {
        expect((await post(admin.page, path, {action: operation, _csrf: csrf, ...fields})).status())
          .toBe(422);
      }
      const [unchanged] = await query(
        'SELECT count(*)::int AS count FROM audit_log WHERE target=$1 AND action=$2',
        [target.id, auditAction],
      );
      expect(unchanged.count).toBe(0);

      await action(admin.page, selector, {body, reason}, operation);
      await expect(admin.page).toHaveURL(new RegExp(`${path}$`));
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
        const confirm = await mailLink(page, nextEmail, /https:\/\/localhost:18443\/email-change\/confirm\/[a-f0-9]{64}/, 85_000);
        await mailLink(page, target.email, /https:\/\/localhost:18443\/email-change\/revert\/[a-f0-9]{64}/, 85_000);
        await page.goto(confirm);
        await submit(page);
        const [pending] = await query(
          `SELECT a.email, c.confirmed_at IS NOT NULL AS confirmed, c.applied_at IS NULL AS waiting
           FROM accounts a JOIN account_email_changes c ON c.account_id=a.id WHERE a.id=$1`,
          [target.id],
        );
        expect(pending).toEqual({email: target.email, confirmed: true, waiting: true});
        // Advance the stored deadline in the disposable database. The real
        // worker has to apply the verified request and keep the address verified.
        await query("UPDATE account_email_changes SET apply_after=now()-interval '1 second' WHERE account_id=$1", [target.id]);
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
    const csrf = await admin.page.locator('form:has(button[value=account-status]) input[name=_csrf]').inputValue();
    expect((await post(admin.page, `/admin/players/${actor.id}`, {
      action: 'account-status', body: 'suspended', reason, _csrf: csrf,
    })).status()).toBe(403);

    await query("UPDATE account_roles SET role_id=(SELECT id FROM roles WHERE key='security_admin') WHERE account_id=$1", [actor.id]);
    expect((await admin.page.request.get(path, {maxRedirects: 0})).status()).toBe(403);
    expect((await post(admin.page, path, {
      action: 'note', body: 'Unauthorized note', reason, _csrf: csrf,
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
  const target = await seedAccount(info.project.metadata.locale);
  const admin = await isolatedAdmin(browser, info);
  try {
    const [actor] = await query('SELECT id FROM accounts WHERE email=$1', [admin.account.email]);
    const [role] = await query('INSERT INTO roles(key) VALUES($1) RETURNING id', [unique()]);
    await query("INSERT INTO role_permissions(role_id,permission) VALUES($1,'account.flags.edit')", [role.id]);
    await rolePage(admin.page, role.id);
    const selector = 'form:has(button[value=grant])';
    const csrf = await admin.page.locator(selector).locator('input[name=_csrf]').inputValue();
    const form = {action: 'grant', account: target.id, role: role.id, reason, _csrf: csrf};
    await csrfRejected(admin.page, '/admin/roles', form);
    expect((await post(admin.page, '/admin/roles', {...form, reason: ''})).status()).toBe(422);
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
  } finally {
    await admin.context.close();
  }
});
