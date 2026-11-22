import { seededPlayer } from '../outbox-helpers.mjs';
import { login } from '../helpers.mjs';
import { blockedBy, enqueue, expect, query, test, waitForJob } from './worker.mjs';

const recurring = [
  'admin.control.expire',
  'cleanup.deaths',
  'cleanup.game-logins',
  'cleanup.game-ticket-budgets',
  'cleanup.hunting-places',
  'cleanup.journal-payloads',
  'cleanup.mfa',
  'cleanup.passkeys',
  'cleanup.rate-limits',
  'cleanup.sessions',
  'cleanup.support',
  'cleanup.support-tickets',
  'cleanup.tokens',
  'cleanup.world-kills',
  'community.today.retention',
  'identity.account-closure-audit',
  'identity.email-change',
  'identity.finalize-terminations',
  'league.retention',
  'referrals.process',
  'stats.death-retention',
  'stats.poll',
  'stats.v2-retention',
  'stats.world-sample-retention',
].sort();

test('two real workers schedule and claim every locally configured recurring job', async ({ jobs }) => {
  const workers = [jobs.start(), jobs.start()];
  await Promise.all(workers.map(worker => worker.ready));
  await expect.poll(async () => (await query(
    "SELECT DISTINCT kind FROM outbox WHERE schedule_slot IS NOT NULL AND state='done' ORDER BY kind",
  )).map(row => row.kind)).toEqual(recurring);
  expect(await query(
    `SELECT kind,schedule_slot FROM outbox WHERE schedule_slot IS NOT NULL
     GROUP BY kind,schedule_slot HAVING count(*)<>1`,
  )).toEqual([]);
  // League uses a bounded 100 ms advisory lock across adjacent one-second
  // slots. Same-slot uniqueness holds even if those distinct jobs retry.
  expect(await query(
    `SELECT kind,schedule_slot FROM outbox
     WHERE schedule_slot IS NOT NULL AND kind<>'league.retention' AND attempts<>1 AND state='done'`,
  )).toEqual([]);
});

test('expired sessions are retried with database backoff and drained under SIGTERM', async ({ browser, page, jobs }, info) => {
  const lang = info.project.metadata.locale;
  const player = await seededPlayer(page, lang);
  const expired = await browser.newContext({ locale: lang, viewport: info.project.use.viewport, ignoreHTTPSErrors: true, baseURL: process.env.PORTAL_E2E_ORIGIN, userAgent: 'Worker expired browser' });
  try {
    const oldPage = await expired.newPage();
    await login(oldPage, lang, player);
    const sessions = `/${lang}/account/sessions`;
    expect((await oldPage.goto(sessions)).status()).toBe(200);
    const [session] = await query(
      `UPDATE web_sessions SET idle_expires_at=clock_timestamp()-interval '1 second'
       WHERE account_id=$1 AND user_agent_summary=$2 RETURNING encode(id_hash,'hex') AS digest`,
      [player.id, 'Worker expired browser'],
    );
    expect(Boolean(session)).toBe(true);
    const expiredRows = () => query("SELECT 1 FROM web_sessions WHERE id_hash=decode($1,'hex')", [session.digest]);

    const id = await enqueue('cleanup.sessions');
    const locker = await jobs.db.connect();
    try {
      await locker.query('BEGIN');
      await locker.query('LOCK TABLE web_sessions IN ACCESS EXCLUSIVE MODE');
      const worker = jobs.start();
      await worker.ready;
      await waitForJob(id, 'running', 1);
      const blocked = await blockedBy(locker, [worker], 'DELETE FROM web_sessions');
      for (const { pid } of blocked) {
        expect(await query('SELECT pg_terminate_backend($1, 1000) AS killed', [pid])).toEqual([{ killed: true }]);
      }
      const { backoff } = await waitForJob(id, 'pending', 1);
      expect(backoff).toBeGreaterThanOrEqual(1);
      expect(backoff).toBeLessThanOrEqual(2);

      await waitForJob(id, 'running', 2);
      await blockedBy(locker, [worker], 'DELETE FROM web_sessions');
      // SIGTERM is sent only after this attempt is observed inside PostgreSQL.
      expect(worker.child.kill('SIGTERM')).toBe(true);
      await locker.query('COMMIT');
      expect(await worker.exited).toEqual([0, null]);
    } finally {
      await locker.query('ROLLBACK');
      locker.release();
    }
    await waitForJob(id, 'done', 2);
    expect(await expiredRows()).toEqual([]);

    const denied = await oldPage.request.get(sessions, { maxRedirects: 0 });
    expect(denied.status()).toBe(303);
    expect(denied.headers().location).toBe(`/${lang}/login`);
    expect((await page.goto(sessions)).status()).toBe(200);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(lang === 'pl' ? 'Sesje web' : 'Web sessions');
  } finally {
    await expired.close();
  }
});

test('a persistent PostgreSQL failure exhausts the job and leaves a healthy sibling runnable', async ({ jobs }) => {
  const id = await enqueue('cleanup.passkeys', 2);
  const healthy = await enqueue('cleanup.mfa');
  // A real undefined-relation error across two attempts. Restore the isolated
  // fixture schema after the worker has exited so statements cannot race DDL.
  await query('ALTER TABLE passkey_ceremonies RENAME TO worker_jobs_passkey_ceremonies');
  const worker = jobs.start();
  try {
    await worker.ready;
    await waitForJob(id, 'dead', 2);
    await waitForJob(healthy, 'done', 1);
    expect(await query(
      `SELECT last_error LIKE '%42P01%' OR last_error LIKE '%does not exist%' AS missing
       FROM outbox WHERE id=$1`,
      [id],
    )).toEqual([{ missing: true }]);
    const fence = await enqueue('cleanup.mfa');
    await waitForJob(fence, 'done', 1);
    await waitForJob(id, 'dead', 2);
  } finally {
    worker.child.kill('SIGTERM');
    expect(await worker.exited).toEqual([0, null]);
    await query('ALTER TABLE worker_jobs_passkey_ceremonies RENAME TO passkey_ceremonies');
  }
});

test('two workers retain recent tickets and remove an aged closed ticket once', async ({ page, jobs }, info) => {
  const lang = info.project.metadata.locale;
  const player = await seededPlayer(page, lang);
  const subjects = ['Expired worker fixture', 'Recent worker fixture'];
  const rows = await query(
    `INSERT INTO tickets(account_id,request_key,category,subject,status,closed_at)
     SELECT $1,uuidv7(),'other',subject,'closed',now()-age
     FROM unnest($2::text[],ARRAY[interval '181 days',interval '179 days']) AS t(subject,age)
     RETURNING id,subject`, [player.id, subjects],
  );
  const old = rows.find(row => row.subject === subjects[0]);
  const oldPath = `/${lang}/support/tickets/${old.id}`;
  expect((await page.goto(oldPath)).status()).toBe(200);
  await expect(page.locator('main')).toContainText(subjects[0]);

  const id = await enqueue('cleanup.support-tickets');
  const workers = [jobs.start(), jobs.start()];
  await Promise.all(workers.map(worker => worker.ready));
  await waitForJob(id, 'done', 1);
  expect(await query('SELECT subject FROM tickets WHERE account_id=$1', [player.id])).toEqual([{ subject: subjects[1] }]);
  const response = await page.request.get(oldPath, { maxRedirects: 0 });
  expect(response.status()).toBe(404);
  const recent = rows.find(row => row.subject === subjects[1]);
  expect((await page.goto(`/${lang}/support/tickets/${recent.id}`)).status()).toBe(200);
  await expect(page.locator('main')).toContainText(subjects[1]);
});
