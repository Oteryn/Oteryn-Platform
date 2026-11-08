import {test as base,expect} from '@playwright/test';
export {expect};

export const test=base.extend({
 browserDiagnostics:[async({context,page},use,info)=>{
   const errors=[];
   const diagnostics=[];
   const watch=screen=>{
     screen.on('pageerror',error=>errors.push(error.message));
     screen.on('response',response=>{
       if(response.status()>=500&&response.status()!==503)errors.push(`HTTP ${response.status()}: ${new URL(response.url()).pathname}`);
     });
     screen.on('console',message=>{
       if(message.type()!=='error')return;
       // Negative authentication/ownership cases intentionally navigate to 4xx;
       // cold Game reads explicitly assert 503. Destination sweeps additionally
       // require the exact status and reject failed subordinate resources.
       if(/^Failed to load resource: the server responded with a status of (4\d\d|503) \([^)]+\)$/.test(message.text())) {
         diagnostics.push({path:new URL(message.location().url||'https://localhost/').pathname,message:message.text()});
       }else errors.push(message.text());
     });
   };
   watch(page);context.on('page',watch);
   await use();
   await info.attach('browser-diagnostics',{body:JSON.stringify({errors,expectedHttpDiagnostics:diagnostics}),contentType:'application/json'});
   expect(errors,'unexpected browser console/script/server errors').toEqual([]);
 },{auto:true}],
});
