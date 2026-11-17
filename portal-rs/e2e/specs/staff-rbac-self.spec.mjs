import {test,expect} from '../fixtures.mjs';
import {staff,role,membership,rolePage,audit,query} from '../staff-rbac-helpers.mjs';
test('admin cannot grant itself a role and revokes its secondary role while retaining remaining permissions',async({browser},info)=>{
 const s=await staff(browser,info);
 try{
  const id=await role(s.page);
  const actor=(await query("SELECT actor FROM audit_log WHERE action='rbac.role.created' AND target=$1",[id]))[0].actor;
  // Four eyes (plan §5.4): a role manager never grants a role to itself.
  await rolePage(s.page,id);
  const token=await s.page.locator('form:has(input[name=account]) input[name=_csrf]').first().inputValue();
  const refused=await s.page.request.post('/admin/roles',{form:{_csrf:token,account:actor,role:id,action:'grant'},headers:{origin:new URL(s.page.url()).origin},maxRedirects:0});
  expect(refused.status(),'self grant refused').toBe(403);
  expect(Number((await query('SELECT count(*) n FROM account_roles WHERE account_id=$1 AND role_id=$2',[actor,id]))[0].n),'refused self grant persists nothing').toBe(0);
  // A membership granted by someone else is still revocable by its holder.
  await query('INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)',[actor,id]);
  await membership(s.page,actor,id,'revoke');
  expect((await s.page.goto('/admin/roles')).status(),'remaining administrator permission survives self revocation').toBe(200);
  await audit(s.page,'rbac.role.created',id);await audit(s.page,'rbac.role.revoked',actor);
 }finally{await s.context.close();}
});
