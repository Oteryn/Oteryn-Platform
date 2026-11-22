import { expect } from '@playwright/test';
import { pool } from '../helpers.mjs';

const identifier = value => '"' + value.replaceAll('"', '""') + '"';

export function database(db = pool()) {

  async function snapshot(sessionHash) {
    const client = await db.connect();
    try {
      await client.query('BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY');
      const { rows: tables } = await client.query(`
        SELECT n.nspname AS schema, c.relname AS name
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE c.relkind = 'r' AND n.nspname !~ '^pg_' AND n.nspname <> 'information_schema'
        ORDER BY n.nspname, c.relname
      `);
      const { rows: [session] } = sessionHash ? await client.query(`
        SELECT account_id, (extract(epoch FROM last_seen_at) * 1000000)::bigint AS last_seen
        FROM public.web_sessions WHERE id_hash = $1
      `, [sessionHash]) : { rows: [] };
      const { rows: [activity] } = session?.account_id ? await client.query(`
        SELECT (extract(epoch FROM last_activity) * 1000000)::bigint AS last_activity,
               (extract(epoch FROM updated_at) * 1000000)::bigint AS updated_at,
               encode(sha256(convert_to((to_jsonb(t) - ARRAY['last_activity', 'updated_at'])::text, 'UTF8')), 'hex') AS stable
        FROM public.referral_activity t WHERE account_id = $1
      `, [session.account_id]) : { rows: [] };
      const nonAuditTables = tables.filter(({ schema, name }) =>
        schema !== 'public' || !['audit_log', 'oauth_security_events'].includes(name));
      const values = [];
      const parameter = value => '$' + values.push(value);
      const statements = nonAuditTables.map(({ schema, name }) => {
        const relation = `${identifier(schema)}.${identifier(name)}`;
        const table = parameter(relation);
        const ownReferral = schema === 'public' && name === 'referral_activity' && session?.account_id;
        const ownSession = schema === 'public' && name === 'web_sessions' && sessionHash;
        const rowData = ownSession ? `CASE WHEN id_hash = ${parameter(sessionHash)} THEN
          to_jsonb(t) - ARRAY['last_seen_at', 'updated_at', 'idle_expires_at']
          ELSE to_jsonb(t) END` : 'to_jsonb(t)';
        return `
          SELECT ${table}::text AS relation,
                 encode(sha256(convert_to(COALESCE(string_agg(digest, '' ORDER BY digest), ''), 'UTF8')), 'hex') AS digest
          FROM (
            SELECT encode(sha256(convert_to((${rowData})::text, 'UTF8')), 'hex') AS digest
            FROM ONLY ${relation} t
            ${ownReferral ? `WHERE account_id <> ${parameter(session.account_id)}` : ''}
          ) rows
        `;
      });
      const { rows: fingerprints } = await client.query(statements.join(' UNION ALL '), values);
      const state = new Map(fingerprints.map(({ relation, digest }) => [relation, digest]));
      const { rows: auditRows } = await client.query(`
        SELECT 'portal:' || id::text AS id, action,
               encode(sha256(convert_to(to_jsonb(t)::text, 'UTF8')), 'hex') AS digest
        FROM public.audit_log t
        UNION ALL
        SELECT 'oauth:' || id::text AS id, event AS action,
               encode(sha256(convert_to(to_jsonb(t)::text, 'UTF8')), 'hex') AS digest
        FROM public.oauth_security_events t
      `);
      const audit = new Map(auditRows.map(({ id, action, digest }) => [id, { action, digest }]));
      await client.query('COMMIT');
      return { state, audit, session, activity };
    } catch (error) {
      await client.query('ROLLBACK');
      throw error;
    } finally {
      client.release();
    }
  }

  return { snapshot, close: () => db.end() };
}

export function assertRefusalState(before, after, denialActions = []) {
  if (JSON.stringify(before.activity) !== JSON.stringify(after.activity)) {
    expect(Boolean(after.activity && after.session && before.session), 'referral write has a tracked session')
      .toBe(true);
    const lastSeen = BigInt(after.session.last_seen);
    const previousActivity = BigInt(before.activity?.last_activity ?? '0');
    expect(lastSeen > BigInt(before.session.last_seen), 'session activity advanced').toBe(true);
    expect(after.activity.last_activity, 'referral activity follows the session refresh')
      .toBe(String(lastSeen > previousActivity ? lastSeen : previousActivity));
    expect(BigInt(after.activity.updated_at) >= lastSeen, 'referral update follows session activity').toBe(true);
    if (before.activity) {
      expect(after.activity.stable, 'referral row identity and other fields are unchanged').toBe(before.activity.stable);
    }
  }
  const relations = new Set([...before.state.keys(), ...after.state.keys()]);
  const changed = [...relations].filter(table => before.state.get(table) !== after.state.get(table));

  const changedAudit = [...before.audit].filter(([id, row]) => row.digest !== after.audit.get(id)?.digest);

  const addedActions = [...after.audit]
    .filter(([id]) => !before.audit.has(id))
    .map(([, row]) => row.action)
    .sort();
  expect({
    changedNonAuditTables: changed,
    changedOrRemovedAuditRows: changedAudit.length,
    addedAuditActions: addedActions,
  }, 'refusal database effects').toEqual({
    changedNonAuditTables: [],
    changedOrRemovedAuditRows: 0,
    addedAuditActions: [...denialActions].sort(),
  });
}
