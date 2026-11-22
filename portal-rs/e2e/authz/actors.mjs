import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { expect } from '@playwright/test';
import { login, password } from '../helpers.mjs';
import { loginWithRecoveryCode, query } from '../staff-rbac-helpers.mjs';

export function actorDefinitions(roles) {
  return [
    { key: 'anonymous', permissions: [] },
    { key: 'player', permissions: [] },
    { key: 'restricted-player', permissions: [], restricted: true },
    ...Object.entries(roles).map(([key, permissions]) => ({ key, permissions, role: key })),
    { key: 'custom', permissions: ['admin.access'], role: `matrix_${randomUUID().replaceAll('-', '')}`, custom: true },
  ];
}

function seed(entity, input) {
  if (process.env.OTERYN_ENV !== 'testing' || !process.env.PORTAL_E2E_BINARY) {
    throw new Error('Authorization fixtures require the harness testing environment and PORTAL_E2E_BINARY');
  }
  try {
    return JSON.parse(execFileSync(process.env.PORTAL_E2E_BINARY,
      ['seed', entity, JSON.stringify(input)], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }));
  } catch {
    // The invocation and child output contain disposable credentials.
    throw new Error(`Harness seed failed for ${entity}`);
  }
}

export async function openActor(browser, info, definition, sanctionIssuer) {
  const locale = info.project.metadata.locale;
  const context = await browser.newContext({
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443', ignoreHTTPSErrors: true, locale,
    viewport: info.project.use.viewport,
  });
  try {
    const page = await context.newPage();
    if (definition.key === 'anonymous') return { context, page };

    const account = { email: `matrix-${randomUUID()}@example.invalid`, password: password() };
    const { accountId } = seed('account', { ...account, locale });
    if (definition.role) {
      if (definition.custom) {
        const [{ id }] = await query('INSERT INTO roles(key) VALUES ($1) RETURNING id', [definition.role]);
        await query('INSERT INTO role_permissions(role_id,permission) VALUES ($1,$2)',
          [id, definition.permissions[0]]);
      }
      const [{ role_id: roleId }] = await query(`
        INSERT INTO account_roles(account_id,role_id)
        SELECT $1,id FROM roles WHERE key=$2
        RETURNING role_id
      `, [accountId, definition.role]);
      const roles = await query('SELECT permission FROM role_permissions WHERE role_id=$1', [roleId]);
      expect(roles.map(row => row.permission).sort(), `${definition.key} catalog permissions`)
        .toEqual([...definition.permissions].sort());
      const { recoveryCodes } = seed('account-mfa', { accountId, password: account.password });
      await loginWithRecoveryCode(page, locale, account, recoveryCodes[0]);
    } else {
      await login(page, locale, account);
    }

    if (definition.restricted) {
      await query(`
        INSERT INTO sanctions (account_id, category, status, public_reason, effective_at, expires_at, created_by)
        VALUES ($1,'restriction','active','Local authorization matrix',now(),now()+interval '1 hour',$2)
      `, [accountId, sanctionIssuer]);
    }
    return { context, page, accountId };
  } catch (error) {
    await context.close();
    throw error;
  }
}
