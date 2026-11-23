import { login, submit } from '../helpers.mjs';
import { after, before, expect, formPost, one, redirected, sweep, test, text } from './fixtures.mjs';

test('account closure grace gates the worker’s first CloseAccount command', async ({ page, lang, db, seed }) => {
  const account = await seed();
  await login(page, lang, account);
  await page.goto(`/${lang}/account/sudo`);
  await submit(page, 'main form', { current_password: account.password });
  const path = `/${lang}/account/privacy/delete`;
  expect((await page.goto(path)).status()).toBe(200);
  redirected(await formPost(page, 'main form', { current_password: account.password, confirmation: 'TERMINATE' }),
    303, `/${lang}/login`);

  const pending = await one(db, `SELECT id,cancelled_at,finalized_at,
    extract(epoch FROM scheduled_for-created_at)::int AS lifetime FROM account_terminations WHERE account_id=$1`, [account.id]);
  expect(pending).toMatchObject({
    cancelled_at: null, finalized_at: null,
    lifetime: Number(process.env.PORTAL_ACCOUNT_TERMINATION_GRACE_DAYS ?? 14) * 86400,
  });
  await login(page, lang, account);
  const move = seconds => one(db, `WITH deadline AS (SELECT clock_timestamp()+make_interval(secs=>$3) AS expires)
    UPDATE account_terminations SET scheduled_for=deadline.expires,created_at=deadline.expires-make_interval(secs=>$4)
    FROM deadline WHERE id=$1 AND account_id=$2 AND cancelled_at IS NULL AND finalized_at IS NULL RETURNING id`,
    [pending.id, account.id, seconds, pending.lifetime]);
  const commands = async () => (await db.query(`SELECT c.command FROM plat3_closure_commands c
    WHERE c.termination_id=$1 AND c.command='CloseAccount'`, [pending.id])).rows;

  await move(before);
  expect((await page.goto(path)).status()).toBe(200);
  await expect(page.locator('main [role=status]')).toHaveText(text(lang,
    'Deletion is scheduled. You can cancel before finalization.',
    'Usunięcie konta jest zaplanowane. Możesz je anulować przed zakończeniem.'));
  await expect(page.locator('form[action$="/cancel"] button')).toHaveText(text(lang,
    'Cancel deletion', 'Anuluj usunięcie'));
  await sweep(db, 'identity.finalize-terminations', commands, []);

  await move(after);
  await expect.poll(commands).toEqual([{ command: 'CloseAccount' }]);
  // A durable Game intent is the Platform worker's effect at this boundary.
  // Cancellation is still supported until finalization commits on Game evidence.
});
