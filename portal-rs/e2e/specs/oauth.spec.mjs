import {test,expect} from '../fixtures.mjs';
import crypto from 'node:crypto';
import {register,login,submit} from '../helpers.mjs';
import {assertShell} from '../shell.mjs';

test('OAuth consent, CSRF, PKCE, single-use code and refresh rotation',async({page},info)=>{
 const lang=info.project.metadata.locale;
 const account=await register(page,lang);await login(page,lang,account);
 const verifier=crypto.randomBytes(32).toString('base64url');
 const redirect='https://localhost:18443/__oauth/callback';
 const state=crypto.randomBytes(24).toString('hex');
 const query={response_type:'code',client_id:'e2e-local',redirect_uri:redirect,scope:'game:ticket',state,code_challenge:crypto.createHash('sha256').update(verifier).digest('base64url'),code_challenge_method:'S256'};
 const response=await page.goto(`/oauth/authorize?${new URLSearchParams(query)}`);
 expect(response.status()).toBe(200);await assertShell(page);
 // The local test client owns this callback, outside the portal router.
 await page.route('**/__oauth/callback?**',route=>route.fulfill({status:200,contentType:'text/plain',body:'Local client received the authorization response.'}));
 await submit(page,'main form');
 await expect(page).toHaveURL(/\/__oauth\/callback\?/);
 const result=new URL(page.url());expect(result.searchParams.get('state')).toBe(state);
 const code=result.searchParams.get('code');expect(code).toBeTruthy();
 const form={grant_type:'authorization_code',client_id:'e2e-local',redirect_uri:redirect,code,code_verifier:verifier};
 const token=await page.request.post('/oauth/token',{form});expect(token.status()).toBe(200);
 expect(token.headers()['cache-control']).toBe('no-store');
 const issued=await token.json();expect(issued.access_token).toBeTruthy();expect(issued.refresh_token).toBeTruthy();
 expect((await page.request.post('/oauth/token',{form})).status()).toBe(400);
 const refresh={grant_type:'refresh_token',client_id:'e2e-local',refresh_token:issued.refresh_token,scope:'game:ticket'};
 const rotated=await page.request.post('/oauth/token',{form:refresh});expect(rotated.status()).toBe(200);
 expect((await rotated.json()).refresh_token).not.toBe(issued.refresh_token);
 expect((await page.request.post('/oauth/token',{form:refresh})).status()).toBe(400);
});
