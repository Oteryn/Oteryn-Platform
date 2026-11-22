import crypto from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {test,expect} from '../fixtures.mjs';
import {csrfRejected,login,mailLink,password,register,submit} from '../helpers.mjs';
import {audit,enrollMfa,loginWithRecoveryCode,query} from '../staff-rbac-helpers.mjs';
import {cookie,deviceCookie,hash} from '../security/auth-fixtures.mjs';

test.use({trace:'off',screenshot:'off'});

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

async function send(page,email,roles,lang) {
  const action=`${manage}?lang=${lang}`;
  const form=page.locator(`form[action="${action}"]`);
  await csrfRejected(page,action,{email,role:roles[0].id});
  await form.getByRole('textbox').fill(email);
  for(const {key} of roles) {
    await form.getByRole('checkbox',{name:key,exact:true}).check();
  }
  const response=page.waitForResponse(r=>r.request().method()==='POST' && new URL(r.url()).pathname===manage);
  await form.getByRole('button').click();
  const result=await response;
  expect(result.status()).toBe(303);
  expect(result.headers().location).toBe(action);
  const entry=page.locator('article').filter({hasText:email});
  for(const {key} of roles) await expect(entry).toContainText(key);
  const [invite]=await query('SELECT id FROM staff_invites WHERE email=$1',[email]);
  return invite.id;
}

async function invitation(page,email,lang) {
  const match=new RegExp(`${origin().replace(/[.*+?^${}()|[\]\\]/g,'\\$&')}/${lang}/staff-invite\\?token=(?:3D)?[a-f0-9]{64}`);
  const link=await mailLink(page,email,match);
  const [{expiry}]=await query(
    "SELECT to_char(expires_at AT TIME ZONE 'UTC',$2) AS expiry FROM staff_invites WHERE email=$1",
    [email,`${lang==='pl'?'DD.MM.YYYY':'YYYY-MM-DD'} HH24:MI:SS "UTC"`],
  );
  const stamp=new RegExp(expiry.replace(/\./g,'\\.'));
  expect(await mailLink(page,email,stamp)).toBe(expiry);
  return link;
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

async function expectInviteQuota(page,path,form,unlimitedStatus,policy) {
  const snapshot=async()=>(await query(
    `SELECT count(DISTINCT key_hash) FILTER (WHERE limiter_name=$1)::int AS source_keys,
            count(*) FILTER (WHERE limiter_name=$1 AND count>12)::int AS source_denials,
            count(*) FILTER (WHERE limiter_name=$2 AND count>12)::int AS account_denials
     FROM web_rate_limits WHERE limiter_name IN ($1,$2)`,
    [policy,`${policy}-account`],
  ))[0];
  const before=await snapshot();
  let attempts=0;
  await expect.poll(async()=>{
    for(let attempt=0;attempt<13;attempt++){
      const response=await page.request.post(path,{
        form,headers:{origin:origin(),'x-portal-e2e-invite-peer':`127.0.0.${19+(++attempts)}`},maxRedirects:0,
      });
      if(response.status()===429){
        expect(Number(response.headers()['retry-after'])).toBeGreaterThan(0);
        return response.status();
      }
      expect(response.status()).toBe(unlimitedStatus);
    }
    return null;
  }).toBe(429);
  const after=await snapshot();
  expect(after.source_keys-before.source_keys,'each request used a distinct source budget').toBe(attempts);
  expect(after.source_denials,'the source policy did not refuse the request').toBe(before.source_denials);
  expect(after.account_denials,'the account policy refused the request').toBe(before.account_denials+1);
}

test('staff invitations reach verified registered and existing accounts through email and MFA',async({browser,page},info)=>{
  test.setTimeout(300_000);
  const lang=info.project.metadata.locale;
  const browserLanguage=lang==='pl' ? 'en' : 'pl';
  const manageLocalized=`${manage}?lang=${lang}`;
  const staff=await browser.newContext({
    baseURL:origin(),ignoreHTTPSErrors:true,locale:browserLanguage,viewport:info.project.use.viewport,
  });
  try {
    const manager=seedAccount(lang);
    const existing=seedAccount(lang);
    const [support]=await query("SELECT id,key FROM roles WHERE key='support'");
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
    await admin.locator(`a[href="${manage}?lang=${browserLanguage}"]`).click();
    await expect(admin.locator('html')).toHaveAttribute('lang',browserLanguage);
    expect((await admin.goto(`/admin/console/access?lang=${lang}`)).status()).toBe(200);
    await admin.locator(`a[href="${manageLocalized}"]`).click();
    await expect(admin.locator('html')).toHaveAttribute('lang',lang);
    await expect(admin.getByRole('heading',{
      name:lang==='pl' ? 'Zaproszenia do zespołu' : 'Staff invitations',exact:true,
    })).toBeVisible();
    const fresh=address();
    const freshId=await send(admin,fresh,[support],lang);
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

    const existingId=await send(admin,existing.email,[support],lang);
    const existingLink=await invitation(page,existing.email,lang);
    await visit(page,existingLink,403);
    expect(await memberships(existing.email)).toEqual([]);
    await login(page,lang,existing);
    const existingCodes=await enrollMfa(page,lang,existing);
    await login(page,lang,existing);
    const mfaChallenge=new RegExp(`/${lang}/mfa/challenge$`);
    await expect(page).toHaveURL(mfaChallenge);
    await submit(page,'main form[action$="/mfa/challenge"]',{code:existingCodes[0],remember:true});
    const deviceHash=hash(await cookie(page.context(),deviceCookie));
    const remembered=await query(
      "SELECT 1 FROM account_mfa_tokens WHERE account_id=$1 AND token_hash=$2 AND purpose='trusted'",
      [existing.id,deviceHash],
    );
    expect(remembered).toHaveLength(1);
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
      const beforePromotion=await copied.newPage();
      await login(beforePromotion,lang,existing);
      await expect(beforePromotion).toHaveURL(mfaChallenge);
    } finally {
      await copied.close();
    }
    expect((await page.goto('/admin')).status()).toBe(200);
    expect((await page.goto('/admin/roles')).status()).toBe(403);

    const revokedEmail=address();
    const revokedId=await send(admin,revokedEmail,[support],lang);
    const revokedLink=await invitation(page,revokedEmail,lang);
    const revokePath=`${manage}/${revokedId}/revoke`;
    const revoked=admin.waitForResponse(r=>r.request().method()==='POST'
      && new URL(r.url()).pathname===revokePath
      && new URLSearchParams(r.request().postData()).has('_csrf'));
    await submit(admin,`form[action="${revokePath}?lang=${lang}"]`);
    const revokeResponse=await revoked;
    expect(revokeResponse.status()).toBe(303);
    expect(revokeResponse.headers().location).toBe(manageLocalized);
    await expect(admin.locator('article').filter({hasText:revokedEmail})).toHaveCount(0);
    await visit(page,revokedLink,403);

    await audit(admin,'staff.invite.created',freshId);
    await audit(admin,'staff.invite.accepted',freshId);
    await audit(admin,'staff.invite.accepted',existingId);
    await audit(admin,'staff.invite.revoked',revokedId);
    expect((await admin.goto(manageLocalized)).status()).toBe(200);
    const manageCsrf=await admin.locator(`form[action="${manageLocalized}"] input[name="_csrf"]`).inputValue();
    await expectInviteQuota(admin,manageLocalized,{_csrf:manageCsrf},400,"staff-invite-manage");
    expect((await page.goto(`/${lang}/account/sudo`)).status()).toBe(200);
    const redeemCsrf=await page.locator('main input[name="_csrf"]').first().inputValue();
    await expectInviteQuota(page,`/${lang}/staff-invite`,{_csrf:redeemCsrf,token:'invalid'},403,'staff-invite-accept');
  } finally {
    await staff.close();
  }
});

test('protected invitations retain queued mail until current authority and independent approval are present',async({browser,page},info)=>{
  test.setTimeout(300_000);
  const lang=info.project.metadata.locale;
  const localized=`${manage}?lang=${lang}`;
  const browserOptions={baseURL:origin(),ignoreHTTPSErrors:true,locale:lang,viewport:info.project.use.viewport};
  const issuerContext=await browser.newContext(browserOptions);
  const approverContext=await browser.newContext(browserOptions);
  try {
    const issuer=seedAccount(lang);
    const approver=seedAccount(lang);
    const recipient=seedAccount(lang);
    const roles=await query("SELECT id,key FROM roles WHERE key IN ('platform_admin','support') ORDER BY key");
    const support=roles.find(role=>role.key==='support');
    await query(
      'INSERT INTO account_roles(account_id,role_id) SELECT manager,role FROM unnest($1::uuid[]) manager CROSS JOIN unnest($2::uuid[]) role',
      [[issuer.id,approver.id],roles.map(role=>role.id)],
    );
    const issuerPage=await issuerContext.newPage();
    const approverPage=await approverContext.newPage();
    for(const [actorPage,account] of [[issuerPage,issuer],[approverPage,approver]]) {
      await login(actorPage,lang,account);
      const codes=await enrollMfa(actorPage,lang,account);
      await loginWithRecoveryCode(actorPage,lang,account,codes[0]);
    }
    expect((await issuerPage.goto(localized)).status()).toBe(200);
    const invite=await send(issuerPage,recipient.email,roles,lang);
    const pending=issuerPage.locator('article').filter({hasText:recipient.email});
    const approvalLink=pending.locator('a[href*="?approval="]');
    await expect(approvalLink).toBeVisible();
    const approvalPath=await approvalLink.getAttribute('href');
    const csrf=await issuerPage.locator(`form[action="${localized}"] input[name="_csrf"]`).inputValue();
    const selfApproval=await issuerPage.request.post(localized,{
      form:{_csrf:csrf,approve:invite},headers:{origin:origin()},maxRedirects:0,
    });
    expect(selfApproval.status()).toBe(403);
    const [original]=await query(
      "SELECT o.id,o.state,o.attempts,o.run_after=i.expires_at AS waits_for_approval,i.approved_by FROM outbox o JOIN staff_invites i ON i.id::text=o.idempotency_key WHERE i.id=$1 AND o.kind='email.staff-invite'",
      [invite],
    );
    expect(original).toMatchObject({state:'pending',attempts:0,waits_for_approval:true,approved_by:null});

    const claimBeforeApproval=async()=>{
      // Make this disposable job due so the real worker checks it before approval.
      await query("UPDATE outbox SET run_after=clock_timestamp() WHERE id=$1",[original.id]);
      await expect.poll(async()=>{
        const [job]=await query(
          "SELECT state,attempts,run_after>clock_timestamp() AS deferred FROM outbox WHERE id=$1",
          [original.id],
        );
        return job;
      },{timeout:30_000}).toEqual({state:'pending',attempts:0,deferred:true});
      const mail=await page.request.get('/__mail');
      expect(mail.status()).toBe(200);
      const messages=await mail.json();
      expect(messages.filter(message=>message.to.includes(recipient.email)).length).toBe(0);
      expect(await memberships(recipient.email)).toEqual([]);
    };
    await query('DELETE FROM account_roles WHERE account_id=$1 AND role_id=$2',[issuer.id,support.id]);
    await claimBeforeApproval();
    await query('INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)',[issuer.id,support.id]);
    await claimBeforeApproval();

    expect((await approverPage.goto(approvalPath)).status()).toBe(200);
    const approvalForm=`form[action="${localized}"]:has(input[name="approve"])`;
    await expect(approverPage.locator(approvalForm)).toBeVisible();
    const approvalResponse=approverPage.waitForResponse(r=>r.request().method()==='POST'
      && new URL(r.url()).pathname===manage
      && new URLSearchParams(r.request().postData()).has('_csrf'));
    await submit(approverPage,approvalForm);
    const approved=await approvalResponse;
    expect(approved.status()).toBe(303);
    expect(approved.headers().location).toBe(localized);
    expect(await query('SELECT approved_by FROM staff_invites WHERE id=$1',[invite])).toEqual([{approved_by:approver.id}]);
    const link=await invitation(page,recipient.email,lang);
    await expect.poll(async()=>{
      const [job]=await query('SELECT state,attempts FROM outbox WHERE id=$1',[original.id]);
      return job;
    }).toEqual({state:'done',attempts:1});
    await login(page,lang,recipient);
    const codes=await enrollMfa(page,lang,recipient);
    await loginWithRecoveryCode(page,lang,recipient,codes[0]);
    await visit(page,link,200);
    await submit(page,`main form[action="/${lang}/staff-invite"]`);
    await expect(page.getByText(lang==='pl' ? /Zaproszenie zostało przyjęte/ : /Invitation accepted/)).toBeVisible();
    expect(await memberships(recipient.email)).toEqual(['platform_admin','support']);
    expect((await page.goto('/admin/roles')).status()).toBe(200);
    await audit(issuerPage,'staff.invite.approved',invite);
    await audit(issuerPage,'staff.invite.accepted',invite);
  } finally {
    await Promise.all([issuerContext.close(),approverContext.close()]);
  }
});
