import { query } from './staff-rbac-helpers.mjs';

export function fixtureDatabase() {
  const connectionString = process.env.PORTAL_E2E_DATABASE_URL || process.env.OTERYN_DATABASE_URL;
  const database = URL.parse(connectionString);
  const addressOptions = ['host', 'hostaddr', 'port', 'dbname', 'database'];
  if (!database
      || !['postgres:', 'postgresql:'].includes(database.protocol)
      || !['localhost', '127.0.0.1', '[::1]'].includes(database.hostname)
      || !database.pathname.startsWith('/portal_e2e_')
      || addressOptions.some(option => database.searchParams.has(option))) {
    throw new Error('Staff fixtures require a local portal_e2e_ database');
  }
  return database;
}

// Exercise the real last-role-holder guard even when earlier specs left grants.
export async function withSoleRoleHolder(roleId, accountId, use) {
  const database = fixtureDatabase().href;
  const otherGrants = await query(
    'DELETE FROM account_roles WHERE role_id=$1 AND account_id<>$2 RETURNING to_jsonb(account_roles) AS membership',
    [roleId, accountId], database,
  );
  try {
    return await use();
  } finally {
    await query(
      'INSERT INTO account_roles SELECT * FROM jsonb_populate_recordset(NULL::account_roles,$1::jsonb)',
      [JSON.stringify(otherGrants.map(row => row.membership))], database,
    );
  }
}
