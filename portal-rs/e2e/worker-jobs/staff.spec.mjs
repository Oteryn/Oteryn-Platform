import { portalCommand, seededAccount } from '../outbox-helpers.mjs';
import { loginWithRecoveryCode } from '../staff-rbac-helpers.mjs';
import { enqueue, expect, query, test, waitForJob } from './worker.mjs';

test('expired staff control requests are audited once and visible to a staff browser', async ({ page, jobs }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seededAccount(lang);
  expect(await query(
    `INSERT INTO account_roles(account_id,role_id)
     SELECT $1,id FROM roles WHERE key='platform_admin' RETURNING account_id`,
    [account.id],
  )).toEqual([{ account_id: account.id }]);
  const { recoveryCodes } = JSON.parse(await portalCommand([
    'seed', 'account-mfa', JSON.stringify({ accountId: account.id, password: account.password }),
  ]));
  expect(recoveryCodes.length).toBe(8);
  await loginWithRecoveryCode(page, lang, account, recoveryCodes[0]);

  // The expiry handler works on a retained, previously queued row. The fixture
  // never sends an agent command or offers a signature as authority.
  const [request] = await query(
    `INSERT INTO admin_control_requests(request_id,admin_operation_id,operation,operation_version,
        kind,target,params,params_digest,dry_run,risk_class,actor,reason,approvals,issued_at,expires_at,
        nonce,signer_key_id,canonical_envelope,signature)
     VALUES (uuidv7(),uuidv7(),'worker-expiry-fixture',1,'command','{}','{}',sha256('{}'::bytea),true,'R0',
        $1,'{}','[]',now()-interval '1 hour',now()-interval '1 minute',
        decode(repeat('00',16),'hex'),'worker-expiry-fixture','{}'::bytea,decode(repeat('00',64),'hex'))
     RETURNING request_id, status`,
    [account.id],
  );
  expect(request.status).toBe('queued');
  const id = await enqueue('admin.control.expire');
  const replay = await enqueue('admin.control.expire');
  const workers = [jobs.start(), jobs.start()];
  await Promise.all(workers.map(worker => worker.ready));
  await waitForJob(id, 'done', 1);
  await waitForJob(replay, 'done', 1);

  expect(await query('SELECT status FROM admin_control_requests WHERE request_id=$1', [request.request_id]))
    .toEqual([{ status: 'expired' }]);
  const action = 'admin.control.transition';
  expect(await query(
    'SELECT actor,before,after FROM audit_log WHERE action=$1 AND request_id=$2',
    [action, request.request_id],
  )).toEqual([{ actor: null, before: { status: 'queued' }, after: { status: 'expired' } }]);

  const filter = new URLSearchParams({ lang, operation: action, request_id: request.request_id });
  expect((await page.goto(`/admin/console/audit?${filter}`)).status()).toBe(200);
  await expect(page.locator('main')).toContainText(action);
  await expect(page.locator('main')).toContainText(request.request_id);
});
