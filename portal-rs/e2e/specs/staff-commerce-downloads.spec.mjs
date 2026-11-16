import {test,expect} from '../fixtures.mjs';
import {register,login} from '../helpers.mjs';

test('downloads origin refuses non-staff publication and keeps unknown artifacts absent',async({page},info)=>{
 const lang=info.project.metadata.locale;
 const account=await register(page,lang);await login(page,lang,account);
 // downloads/README.md and downloads/src/http.rs: publication is CLI-only,
 // signed TUF metadata is required and HTTP exposes GET only. There is no admin
 // release/channel pointer form, rollback or unpublish action to drive.
 const target='/updates/stable/metadata/timestamp.json';
 const response=await page.request.post(target,{data:'unauthorized publication'});
 expect(response.status(),'read-only origin refuses publication over HTTP').toBe(405);
 const missing=await page.request.get(`/updates/stable/metadata/missing-${Date.now()}.json`);
 expect(missing.status(),'unknown metadata cannot be served').toBe(404);
});
