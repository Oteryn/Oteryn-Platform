import {test,expect} from '../fixtures.mjs';
import {submit} from '../helpers.mjs';
import {staff,player,action,audit,unique,query} from '../staff-rbac-helpers.mjs';
// SSR controls: support_web.rs::show/forms; player forms: tickets/templates and reports/templates.
test('support staff replies and resolves a ticket visible to its player',async({page,browser},info)=>{
 test.setTimeout(180000);const lang=info.project.metadata.locale;await player(page,lang);const s=await staff(browser,info);const subject=unique(),reply=unique();
 try{
  await s.page.setViewportSize(info.project.use.viewport);
  await page.goto(`/${lang}/support/tickets/create`);await submit(page,'main form',{subject,body:`Question ${subject}`});await expect(page).toHaveURL(/support\/tickets\/[a-f0-9-]+$/);
  const playerPath=new URL(page.url()).pathname,id=playerPath.split('/').at(-1);
  // Paginate the real list; the shared database need not be empty.
  let found=false;for(let n=1;n<=10000;n++){expect((await s.page.goto(`/admin/support/tickets?page=${n}`)).status()).toBe(200);const link=s.page.locator(`a[href="/admin/support/tickets/${id}"]`);if(await link.count()){await link.click();found=true;break;}if(!await s.page.locator(`a[href="/admin/support/tickets?page=${n+1}"]`).count())break;}
  expect(found,'new ticket appears in staff list').toBe(true);
  await action(s.page,'form:has(input[value=reply])',{body:reply,reason:unique()});await expect(s.page.locator('main'),'staff reply persisted').toContainText(reply);
  await action(s.page,'form:has(input[value=ticket_status])',{status:'resolved',reason:unique()});
  await page.goto(playerPath);await expect(page.locator('main'),'player sees staff reply').toContainText(reply);
  expect((await query('SELECT status FROM tickets WHERE id=$1',[id]))[0].status).toBe('resolved');
  // The player template localizes status; compare the actual localized staff label.
  await expect(page.locator('main'),'player sees resolved state').toContainText(lang==='pl'?'Rozwiązane':'Resolved');
  await audit(s.page,'support.ticket_replied',id);await audit(s.page,'support.ticket_status_changed',id);
 }finally{await s.context.close();}
});

test('report moderation issues and lifts a persisted sanction and publishes the outcome to the player',async({page,browser},info)=>{
 test.setTimeout(300000);const lang=info.project.metadata.locale;const account=await player(page,lang);const s=await staff(browser,info);const reason=unique(),outcome=unique(),lift=unique();
 try{
  await s.page.setViewportSize(info.project.use.viewport);
  // Optional support email defaults off; opt in through the real preference form.
  await page.goto(`/${lang}/account/notifications`);await submit(page,'main form',{support_email:true,support_portal:true});await page.reload();await expect(page.locator('[name=support_email]')).toBeChecked();
  await page.goto(`/${lang}/support/reports/create`);await submit(page,'main form',{target_reference:account.id,evidence_summary:reason});await expect(page).toHaveURL(/support\/reports\/[a-f0-9-]+$/);const playerPath=new URL(page.url()).pathname,report=playerPath.split('/').at(-1);
  expect((await s.page.goto('/admin/moderation/reports')).status()).toBe(200);
  expect((await s.page.goto(`/admin/moderation/reports/${report}`)).status()).toBe(200);
  await action(s.page,'form:has(input[value=moderate])',{status:'reviewing',public_outcome:'',moderator_notes:reason});
  await s.page.goto('/admin/moderation/enforcement/create');
  const values={target:account.id,category:'restriction',duration_hours:'1',public_reason:reason,moderator_notes:`Report ${report}`};
  await action(s.page,'form:has(input[value=apply_sanction])',values);await expect(s.page).toHaveURL(/enforcement\/[a-f0-9-]+$/);const sanction=new URL(s.page.url()).pathname.split('/').at(-1);
  const row=(await query('SELECT account_id,status,public_reason,extract(epoch FROM expires_at-effective_at)::int AS duration_seconds FROM sanctions WHERE id=$1',[sanction]))[0];expect(row).toMatchObject({account_id:account.id,status:'active',public_reason:reason,duration_seconds:3600});
  await expect.poll(async()=>{const response=await page.request.get('/__mail');return (await response.json()).some(m=>m.to.includes(account.email)&&m.raw.replace(/=\r?\n/g,'').includes(reason));},{message:'player receives sanction reason through real support mail',timeout:90000}).toBe(true);
  expect(Number((await query('SELECT count(*) n FROM support_enforcement_results r JOIN support_enforcement_commands c USING(operation_id) WHERE c.account_id=$1',[account.id]))[0].n),'Game enforcement remains unavailable without producer receipts').toBe(0);
  await audit(s.page,'support.enforcement_created',sanction);
  await s.page.goto(`/admin/moderation/reports/${report}`);await action(s.page,'form:has(input[value=moderate])',{status:'actioned',public_outcome:outcome,moderator_notes:`Sanction ${sanction}`});
  await page.goto(playerPath);await expect(page.locator('main'),'player sees public enforcement outcome').toContainText(outcome);
  // No player sanction-history HTTP router is composed; Appeals is domain-only.
  // No local fake producer supplies enforcement results. Do not invent a Game receipt:
  // this proves Platform sanction state and public report outcome, not Game execution.
  await s.page.goto(`/admin/moderation/enforcement/${sanction}`);await action(s.page,'form:has(input[value=revoke_sanction])',{public_reason:lift});
  expect((await query('SELECT status FROM sanctions WHERE id=$1',[sanction]))[0].status,'staff lift persists').toBe('revoked');
  await expect.poll(async()=>{const response=await page.request.get('/__mail');return (await response.json()).some(m=>m.to.includes(account.email)&&m.raw.replace(/=\r?\n/g,'').includes(lift));},{message:'player receives lift notification',timeout:90000}).toBe(true);
  await audit(s.page,'support.enforcement_updated',sanction);await audit(s.page,'support.report_status_changed',report);
 }finally{await s.context.close();}
});
