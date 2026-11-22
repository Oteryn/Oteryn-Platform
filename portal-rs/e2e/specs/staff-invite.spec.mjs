import crypto from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {test,expect} from '../fixtures.mjs';
import {csrfRejected,login,mailLink,password,register,submit} from '../helpers.mjs';
import {audit,enrollMfa,loginWithRecoveryCode,query} from '../staff-rbac-helpers.mjs';

const manage='/admin/staff-invites';
const address=()=>`invite-${crypto.randomUUID()}@example.invalid`;
const origin=()=>new URL(process.env.OTERYN_PUBLIC_ORIGIN || 'https://localhost:18443').origin;

function seedAccount(lang) {
  const account={email:address(),password:password(),locale:lang};
  const binary=process.env.PORTAL_E2E_BINARY;
  if(!binary) throw new Error('The local E2E launcher must supply the seed binary');
  let result;
  try {
    result=JSON.parse(execFileSync(binary,['seed','account',JSON.stringify(account)],{
      env:{...process.env,OTERYN_ENV:'testing',OTERYN_DATABASE_URL:process.env.PORTAL_E2E_DATABASE_URL},
      stdio:['ignore','pipe','pipe'],
    }).toString());
  } catch {
    // The invocation includes a disposable password. Keep it out of reporters.
    throw new Error('The disposable account seed failed');
  }
  expect(result.email).toBe(account.email);
  return {...account,id:result.accountId};
}

async function send(page,email,role) {
  const form=page.locator(`form[action="${manage}"]`);
  await csrfRejected(page,manage,{email,role});
  await form.getByRole('textbox').fill(email);
  await form.getByRole('checkbox',{name:'support',exact:true}).check();
  const response=page.waitForResponse(r=>r.request().method()==='POST' && new URL(r.url()).pathname===manage);
  await form.getByRole('button').click();
  expect((await response).status()).toBe(303);
  await expect(page.locator('article').filter({hasText:email})).toContainText('support');
  const [invite]=await query('SELECT id FROM staff_invites WHERE email=$1',[email]);
  return invite.id;
}

async function invitation(page,email,lang) {
  const match=new RegExp(`${origin().replace(/[.*+?^${}()|[\]\\]/g,'\\$&')}/${lang}/staff-invite\\?token=(?:3D)?[a-f0-9]{64}`);
  return mailLink(page,email,match);
}

async function visit(page,link,status) {
  const response=await page.goto(link).catch(()=>null);
  expect(response?.status(),'staff invitation status').toBe(status);
  expect(response.headers()['cache-control']).toBe(status===200 ? 'no-store, max-age=0' : 'no-store');
  expect(response.headers()['referrer-policy']).toBe('strict-origin');
}

async function memberships(email) {
  return (await query(
    "SELECT r.key FROM account_roles ar JOIN roles r ON r.id=ar.role_id JOIN accounts a ON a.id=ar.account_id WHERE a.email=$1 ORDER BY r.key",
    [email],
  )).map(row=>row.key);
}

async function expectInviteQuota(page,path,form,unlimitedStatus) {
  let host=0;
  await expect.poll(async()=>{
    for(let attempt=0;attempt<13;attempt++){
      const response=await page.request.post(path,{
        form,headers:{origin:origin(),'x-forwarded-for':`192.0.2.${++host}`},maxRedirects:0,
      });
      if(response.status()===429){
        expect(Number(response.headers()['retry-after'])).toBeGreaterThan(0);
        return response.status();
      }
      expect(response.status()).toBe(unlimitedStatus);
    }
    return null;
  }).toBe(429);
}

test('staff invitations reach verified registered and existing accounts through email and MFA',async({browser,page},info)=>{
  test.setTimeout(300_000);
  const lang=info.project.metadata.locale;
  const staff=await browser.newContext({
    baseURL:origin(),ignoreHTTPSErrors:true,locale:lang,viewport:info.project.use.viewport,
  });
  try {
    const manager=seedAccount(lang);
    const existing=seedAccount(lang);
    const [support]=await query("SELECT id FROM roles WHERE key='support'");
    // The seed CLI creates verified accounts. Assign this fixture the exact
    // initial memberships it needs to send invitations through the real panel.
    await query(
      "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key IN ('platform_admin','support')",
      [manager.id],
    );
    const admin=await staff.newPage();
    await login(admin,lang,manager);
    const recovery=await enrollMfa(admin,lang,manager);
    await loginWithRecoveryCode(admin,lang,manager,recovery[0]);
    expect((await admin.goto('/admin/roles')).status()).toBe(200);
    await admin.locator(`a[href="${manage}"]`).click();
    await expect(admin.locator('html')).toHaveAttribute('lang',lang);
    await expect(admin.getByRole('heading',{
      name:lang==='pl' ? 'Zaproszenia do zespołu' : 'Staff invitations',exact:true,
    })).toBeVisible();
    expect((await admin.goto(`/admin/console/access?lang=${lang}`)).status()).toBe(200);
    await admin.locator(`a[href="${manage}"]`).click();

    const fresh=address();
    const freshId=await send(admin,fresh,support.id);
    const freshLink=await invitation(page,fresh,lang);
    await visit(page,freshLink,200);
    await expect(page.locator(`main a[href="/${lang}/register"]`)).toBeVisible();
    const registered=await register(page,lang,fresh);
    await login(page,lang,registered);
    await visit(page,freshLink,200);
    await expect(page.locator(`main a[href="/${lang}/mfa"]`)).toBeVisible();
    const mfaRoute=`/${lang}/mfa`;
    expect((await page.goto(mfaRoute)).status()).toBe(200);
    const csrf=await page.locator('form[action$="/mfa/enroll"] input[name="_csrf"]').inputValue();
    const refused=await page.request.post(`/${lang}/staff-invite`,{
      form:{_csrf:csrf,token:new URL(freshLink).searchParams.get('token')},
      headers:{origin:origin()},maxRedirects:0,
    });
    expect(refused.status()).toBe(403);
    expect(await memberships(fresh)).toEqual([]);
    await visit(page,freshLink,200);
    await submit(page,`main form[action="/${lang}/logout"]`);
    await expect(page).toHaveURL(new URL(`/${lang}/`,origin()).href);
    await visit(page,freshLink,200);
    await expect(page.locator(`main a[href="/${lang}/login"]`)).toBeVisible();
    await login(page,lang,registered);
    const freshCodes=await enrollMfa(page,lang,registered);
    await loginWithRecoveryCode(page,lang,registered,freshCodes[0]);
    await visit(page,freshLink,200);
    await submit(page,`main form[action="/${lang}/staff-invite"]`);
    await expect(page.getByText(lang==='pl' ? /Zaproszenie zostało przyjęte/ : /Invitation accepted/)).toBeVisible();
    expect(await memberships(fresh)).toEqual(['support']);
    await visit(page,freshLink,403);

    const existingId=await send(admin,existing.email,support.id);
    const existingLink=await invitation(page,existing.email,lang);
    await visit(page,existingLink,403);
    expect(await memberships(existing.email)).toEqual([]);
    await login(page,lang,existing);
    const existingCodes=await enrollMfa(page,lang,existing);
    await loginWithRecoveryCode(page,lang,existing,existingCodes[0]);
    await visit(page,existingLink,200);
    const copied=await browser.newContext({
      baseURL:origin(),ignoreHTTPSErrors:true,storageState:await page.context().storageState(),
    });
    try {
      const previous=await copied.request.get('/admin',{maxRedirects:0});
      expect(previous.status()).toBe(403);
      await submit(page,`main form[action="/${lang}/staff-invite"]`);
      await expect(page.getByText(lang==='pl' ? /Zaproszenie zostało przyjęte/ : /Invitation accepted/)).toBeVisible();
      expect(await memberships(existing.email)).toEqual(['support']);
      const stale=await copied.request.get('/admin',{maxRedirects:0});
      expect(stale.status()).toBe(303);
      expect(stale.headers().location).toBe('/login');
    } finally {
      await copied.close();
    }
    expect((await page.goto('/admin')).status()).toBe(200);
    expect((await page.goto('/admin/roles')).status()).toBe(403);

    const revokedEmail=address();
    const revokedId=await send(admin,revokedEmail,support.id);
    const revokedLink=await invitation(page,revokedEmail,lang);
    await submit(admin,`form[action="${manage}/${revokedId}/revoke"]`);
    await expect(admin.locator('article').filter({hasText:revokedEmail})).toHaveCount(0);
    await visit(page,revokedLink,403);

    await audit(admin,'staff.invite.created',freshId);
    await audit(admin,'staff.invite.accepted',freshId);
    await audit(admin,'staff.invite.accepted',existingId);
    await audit(admin,'staff.invite.revoked',revokedId);
    expect((await admin.goto(manage)).status()).toBe(200);
    const manageCsrf=await admin.locator(`form[action="${manage}"] input[name="_csrf"]`).inputValue();
    await expectInviteQuota(admin,manage,{_csrf:manageCsrf},400);
    expect((await page.goto(`/${lang}/account/sudo`)).status()).toBe(200);
    const redeemCsrf=await page.locator('main input[name="_csrf"]').first().inputValue();
    await expectInviteQuota(page,`/${lang}/staff-invite`,{_csrf:redeemCsrf,token:'invalid'},403);
  } finally {
    await staff.close();
  }
});
