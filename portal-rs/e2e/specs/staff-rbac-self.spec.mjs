import {test,expect} from '../fixtures.mjs';
import {staff,role,membership,audit,query} from '../staff-rbac-helpers.mjs';
test('admin revokes its secondary role and retains remaining permissions',async({browser},info)=>{
 const s=await staff(browser,info);
 try{
  const id=await role(s.page);
  const actor=(await query("SELECT actor FROM audit_log WHERE action='rbac.role.created' AND target=$1",[id]))[0].actor;
  await membership(s.page,actor,id,'grant');
  await membership(s.page,actor,id,'revoke');
  expect((await s.page.goto('/admin/roles')).status(),'remaining administrator permission survives self revocation').toBe(200);
  await audit(s.page,'rbac.role.created',id);await audit(s.page,'rbac.role.granted',actor);await audit(s.page,'rbac.role.revoked',actor);
 }finally{await s.context.close();}
});
