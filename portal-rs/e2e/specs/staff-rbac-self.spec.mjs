import {test,expect} from '../fixtures.mjs';
import path from 'node:path';
import crypto from 'node:crypto';
import {pool,submit,csrfRejected} from '../helpers.mjs';

test('admin revokes its secondary role and retains remaining permissions',async({browser},info)=>{
 const context=await browser.newContext({storageState:path.join(process.env.PORTAL_E2E_AUTH_DIRECTORY,`staff-${info.project.name}.json`),baseURL:process.env.PORTAL_E2E_ORIGIN||'https://localhost:18443',ignoreHTTPSErrors:true});
 const page=await context.newPage();const db=pool();
 try {
  await page.goto('/admin/roles');
  const key=`staff_rbac_${crypto.randomBytes(10).toString('hex')}`;
  await submit(page,'form:has(input[name=key])',{key,permissions:''});
  await expect(page.locator('main'), 'created role appears in role catalogue').toContainText(key);
  const role=(await db.query('SELECT id FROM roles WHERE key=$1',[key])).rows[0].id;
  // Resolve the authenticated actor from the persisted session, without seeding roles.
  const actor=(await db.query("SELECT actor FROM audit_log WHERE action='rbac.role.created' AND target=$1 ORDER BY created_at DESC LIMIT 1",[role])).rows[0].actor;
  const form=page.locator('form:has(input[name=account])');
  await form.locator('[name=account]').fill(actor);
  await form.locator('[name=role]').selectOption(role);
  await csrfRejected(page,'/admin/roles',{account:actor,role,action:'grant'});
  await form.locator('button[value=grant]').click();
  await expect.poll(async()=>Number((await db.query('SELECT count(*) AS n FROM account_roles WHERE account_id=$1 AND role_id=$2',[actor,role])).rows[0].n),{message:'secondary role grant persists'}).toBe(1);
  await page.goto('/admin/roles');
  await form.locator('[name=account]').fill(actor);
  await form.locator('[name=role]').selectOption(role);
  await csrfRejected(page,'/admin/roles',{account:actor,role,action:'revoke'});
  await form.locator('button[value=revoke]').click();
  await expect.poll(async()=>Number((await db.query('SELECT count(*) AS n FROM account_roles WHERE account_id=$1 AND role_id=$2',[actor,role])).rows[0].n),{message:'self revocation persists'}).toBe(0);
  expect((await page.goto('/admin/roles')).status(),'remaining administrator permission survives self revocation').toBe(200);
 }finally{await db.end();await context.close();}
});
