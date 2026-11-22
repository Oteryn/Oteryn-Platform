import { spawn } from 'node:child_process';
import crypto from 'node:crypto';
import { once } from 'node:events';
import { readFileSync } from 'node:fs';
import { createInterface } from 'node:readline';
import { expect, test as base } from '../fixtures.mjs';
import { pool } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';

export { expect, query };

export const test = base.extend({
  jobs: async ({}, use) => {
    const db = pool();
    const workers = [];
    const env = JSON.parse(readFileSync(process.env.PORTAL_WORKER_JOBS_ENV, 'utf8'));
    try {
      await use({
        db,
        env,
        start: (overrides = {}) => {
          const name = `worker-jobs-${crypto.randomUUID()}`;
          const url = new URL(env.OTERYN_DATABASE_URL);
          url.searchParams.set('application_name', name);
          const child = spawn(process.env.PORTAL_E2E_OUTBOX_BINARY, ['worker'], {
            env: { ...env, ...overrides, OTERYN_DATABASE_URL: url.toString() },
            stdio: ['ignore', 'pipe', 'ignore'],
          });
          const exited = once(child, 'exit');
          // The production ready message follows installation of SIGTERM.
          const ready = new Promise((resolve, reject) => {
            const lines = createInterface({ input: child.stdout });
            lines.on('line', line => {
              if (line.includes('outbox worker started')) resolve();
            });
            lines.once('close', () => reject(new Error('Worker exited before installing its signal handler')));
            child.once('error', () => reject(new Error('Worker could not start')));
          });
          ready.catch(() => {});
          const worker = { name, child, ready, exited };
          workers.push(worker);
          return worker;
        },
      });
    } finally {
      for (const { child } of workers) child.kill('SIGTERM');
      for (const { exited } of workers) await exited;
      await db.end();
    }
  },
});

export async function enqueue(kind, maxAttempts = 5) {
  const [row] = await query(
    `INSERT INTO outbox (kind,payload,idempotency_key,run_after,max_attempts)
     VALUES ($1,'{}',$2,'epoch',$3) RETURNING id`,
    [kind, crypto.randomUUID(), maxAttempts],
  );
  return row.id;
}

export async function status(id) {
  const [row] = await query(
    `SELECT state, attempts, leased_until IS NULL AS released, last_error IS NOT NULL AS failed,
            extract(epoch FROM run_after-updated_at)::float8 AS backoff
     FROM outbox WHERE id=$1`,
    [id],
  );
  return row;
}

export async function waitForJob(id, state, attempts) {
  let row;
  await expect.poll(async () => {
    row = await status(id);
    return row;
  }).toMatchObject({
    state, attempts, released: state !== 'running',
    ...(state === 'running' ? {} : { failed: state === 'pending' || state === 'dead' }),
  });
  return row;
}

// Observing pg_blocking_pids proves the target statement has entered PostgreSQL
// before a signal or backend termination is sent. Query text stays in PostgreSQL.
export async function blockedBy(locker, workers, relation) {
  const [{ pid }] = (await locker.query('SELECT pg_backend_pid() AS pid')).rows;
  let matches;
  await expect.poll(async () => {
    matches = await query(
      `SELECT pid, application_name FROM pg_stat_activity
       WHERE datname=current_database() AND application_name=ANY($1::text[])
         AND $2=ANY(pg_blocking_pids(pid)) AND strpos(query,$3)>0`,
      [workers.map(worker => worker.name), pid, relation],
    );
    return matches.length;
  }, { message: `A worker is blocked in ${relation}` }).toBeGreaterThan(0);
  return matches;
}
