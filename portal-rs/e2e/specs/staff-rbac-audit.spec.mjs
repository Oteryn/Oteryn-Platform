import {test,expect} from '../fixtures.mjs';
import {staff,role,audit,player} from '../staff-rbac-helpers.mjs';
test('audit console filters show actor and target and refuse an ordinary player',async({page,browser},info)=>{
 test.setTimeout(180000);await player(page,info.project.metadata.locale);const s=await staff(browser,info);
 try{const id=await role(s.page);await audit(s.page,'rbac.role.created',id);expect((await page.goto('/admin/console/audit')).status(),'ordinary player cannot read audit evidence').toBe(403);}
 finally{await s.context.close();}
});
