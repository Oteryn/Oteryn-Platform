// TEST SUPPORT ONLY. Synthetic Game zero-character producer, never imported by Rust.
// Exact source fixture bytes remain in identity/termination/fixtures with SOURCE.md.
import fs from 'node:fs';
import crypto from 'node:crypto';
import https from 'node:https';
import http from 'node:http';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const fixturePath=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../crates/identity/src/termination/fixtures/closure.json');
const fixtureBytes=fs.readFileSync(fixturePath);
const hash=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
if(hash(fixtureBytes)!=='22ea70a219875c97d988cd5bf1a74303eb772ef2027dd89b7e22a345033c2f19')throw new Error('Pinned closure fixture changed');
const vectors=JSON.parse(fixtureBytes).vectors;
for(const vector of vectors)if(hash(vector.wire)!==vector.sha256)throw new Error('Pinned fixture wire hash mismatch');
const template=name=>JSON.parse(vectors.find(v=>v.name===name).wire);
export const characterSourceAuthority='oteryn:character-authority:primary';
export function createGameClosureProducer({baseURL,ca,character,portfolio,authority=characterSourceAuthority,controlPort=18777}) {
 if(!['127.0.0.1','localhost'].includes(new URL(baseURL).hostname))throw new Error('Test Game producer requires loopback');
 const keys=crypto.generateKeyPairSync('ed25519');
 const publicKeyHex=keys.publicKey.export({type:'spki',format:'der'}).subarray(-32).toString('hex');
 const keyId='local-closure-fixture';
 const operations=new Map(),accounts=new Map(),faults=new Map();
 let stopped=false,busy=false,timer;
 const request=(route,body,identity=character,sign=false,account)=>new Promise((resolve,reject)=>{
  const raw=typeof body==='string'?body:JSON.stringify(body);
  const headers={'content-type':'application/json','content-length':Buffer.byteLength(raw)};
  if(sign){let signature=crypto.sign(null,Buffer.from(raw),keys.privateKey);if(faults.get(account)==='invalid_signature')signature[0]^=1;headers['x-oteryn-key-id']=keyId;headers['x-oteryn-signature']=signature.toString('hex');}
  const req=https.request(new URL(route,baseURL),{method:'POST',headers,ca,cert:identity.cert,key:identity.key,minVersion:'TLSv1.3',maxVersion:'TLSv1.3',ALPNProtocols:['http/1.1'],timeout:3000},res=>{let result='';res.on('data',part=>{result+=part;});res.on('end',()=>resolve({status:res.statusCode,raw:result}));});
  req.on('error',reject);req.on('timeout',()=>req.destroy(new Error('Game fixture exchange timeout')));req.end(raw);
 });
 const prefix='/internal/v1/game-auth';
 const source={contract_version:2,source_authority:authority};
 async function decide(raw) {
  const intent=JSON.parse(raw),now=Math.floor(Date.now()/1000);
  if(!['CloseAccount','ResolveAccountClosure'].includes(intent.command)||intent.expected_recovery_generation!=='3')return;
  if(operations.has(intent.operation_id)){if(operations.get(intent.operation_id).intent!==raw)throw new Error('Immutable intent conflict');return;}
  if(faults.get(intent.account_id)==='hold')return;
  const lifecycleRead=await request(`${prefix}/account-closure/lifecycle/read`,{contract_version:2,operation:'ReadAccountClosureLifecycleV1',source_authority:authority,account_id:intent.account_id,termination_id:intent.termination_id,recovery_generation:'3',audience:'OTERYN_PLATFORM_CHARACTER_AUTHORITY'});
  if(lifecycleRead.status!==200)return;
  if(Buffer.byteLength(lifecycleRead.raw)>2048)return;
  const lifecycle=JSON.parse(lifecycleRead.raw),phase=intent.command==='CloseAccount'?'CLOSE':intent.decision;
  const fields=['contract_version','operation','source_authority','account_id','termination_id','lifecycle_generation','source_revision','phase','financial_obligations_clear','recovery_generation','observed_at','valid_until'];
  if(Object.keys(lifecycle).join(',')!==fields.join(',')||JSON.stringify(lifecycle)!==lifecycleRead.raw||/[\s\\]/.test(lifecycleRead.raw)||lifecycle.contract_version!==2||lifecycle.operation!=='ReadAccountClosureLifecycleV1')return;
  for(const key of ['lifecycle_generation','source_revision','recovery_generation','observed_at','valid_until'])if(typeof lifecycle[key]!=='string'||! /^[1-9][0-9]*$/.test(lifecycle[key])||BigInt(lifecycle[key])>18446744073709551615n)return;
  if(BigInt(lifecycle.valid_until)>9223372036854775807n||BigInt(lifecycle.valid_until)<=BigInt(lifecycle.observed_at))return;
  if(lifecycle.source_authority!=='OTERYN_PLATFORM_CHARACTER_AUTHORITY'||lifecycle.account_id!==intent.account_id||lifecycle.termination_id!==intent.termination_id||lifecycle.recovery_generation!=='3'||lifecycle.lifecycle_generation!==intent.lifecycle_generation||lifecycle.phase!==phase||Number(lifecycle.observed_at)>Math.floor(Date.now()/1000)||Number(lifecycle.valid_until)<=Math.floor(Date.now()/1000)||Number(lifecycle.valid_until)-Number(lifecycle.observed_at)>5||typeof lifecycle.financial_obligations_clear!=='boolean'||(phase!=='CANCEL'&&!lifecycle.financial_obligations_clear))return;
  const previous=accounts.get(intent.account_id);
  if(intent.command==='ResolveAccountClosure'&&intent.decision==='FINALIZE'&&!operations.has(intent.close_operation_id))return;
  if(intent.command==='CloseAccount'&&previous?.decision==='CANCEL')return;
  if(now<Number(intent.scheduled_for)&&intent.decision!=='CANCEL')return;
  const expired=now>Number(intent.expires_at_source);
  const receipt=template('closed_receipt');receipt.source_authority=authority;receipt.command=intent.command;receipt.operation_id=intent.operation_id;receipt.recovery_generation='3';receipt.decided_at=String(now);
  let projection,lcfa;
  if(expired){receipt.outcome='REJECTED';receipt.result_code='CHAR_CMD_INTENT_EXPIRED';receipt.result_data=null;}
  else {
   const revision=String(Number(previous?.revision||0)+1);
   const state=intent.command==='CloseAccount'?'CLOSED_DETACHED':intent.decision==='CANCEL'?'CANCELLED':'FINALIZED';
   const result=receipt.result_data;Object.assign(result,{account_id:intent.account_id,termination_id:intent.termination_id,lifecycle_generation:intent.lifecycle_generation,closure_state:state,projection_epoch:'3',projection_revision:revision});
   delete result.proof_sha256;result.proof_sha256=hash(JSON.stringify(result));
   const retained=JSON.stringify(receipt);
   projection=template('closure_projection');Object.assign(projection,{source_authority:authority,account_id:intent.account_id,projection_revision:revision,source_observed_at:String(now)});
   Object.assign(projection.closures[0],{termination_id:intent.termination_id,operation_id:intent.operation_id,lifecycle_generation:intent.lifecycle_generation,closure_state:state,receipt_digest:hash(retained),proof_sha256:result.proof_sha256});
   lcfa={contract_version:1,operation:'PublishAccountCharactersV1',source_authority:authority,account_id:intent.account_id,projection_epoch:'3',projection_revision:revision,source_observed_at:String(now),characters:[]};
   accounts.set(intent.account_id,{revision,decision:intent.decision,projection,lcfa});
  }
  operations.set(intent.operation_id,{intent:raw,receipt:JSON.stringify(receipt),account:intent.account_id,projection,lcfa});
 }
 async function tick(){
  if(stopped||busy)return;busy=true;
  try {
   const pending=await request(`${prefix}/character-commands/pending`,{contract_version:2,operation:'ListPendingCharacterCommandsV1',source_authority:authority,after_source_revision:'0',after_operation_id:'00000000-0000-0000-0000-000000000000',max_entries:32});
   if(pending.status!==200)return;
   for(const entry of JSON.parse(pending.raw).pending){const read=await request(`${prefix}/character-commands/read`,{contract_version:2,operation:'ReadCharacterCommandIntentV1',source_authority:authority,issuer_authority:'OTERYN_PLATFORM_CHARACTER_AUTHORITY',command:entry.command,operation_id:entry.operation_id});if(read.status===200)await decide(read.raw);}
   let complete=true;
   for(const operation of operations.values()) {
    if(faults.get(operation.account)==='hold'){complete=false;continue;}
    const receipt=await request(`${prefix}/character-commands/receipt`,operation.receipt,character,true,operation.account);
    if(receipt.status!==200){complete=false;continue;}
    if(!operation.duplicateChecked){const replay=await request(`${prefix}/character-commands/receipt`,operation.receipt,character,true,operation.account);if(replay.status!==200)throw new Error('Receipt replay refused');operation.duplicateChecked=true;}
   }
   for(const [account,state] of accounts) {
    const projection=await request(`${prefix}/account-closure`,state.projection,character,true,account);
    if(projection.status!==200)complete=false;
    if(faults.get(account)==='missing_lcfa'){complete=false;continue;}
    const snapshot=await request(`${prefix}/native-account-characters`,state.lcfa,portfolio);
    if(snapshot.status!==200)complete=false;
   }
   if(complete&&accounts.size){const now=String(Math.floor(Date.now()/1000));await request(`${prefix}/native-account-characters/watermark`,{contract_version:1,operation:'PublishProjectionWatermarkV1',source_authority:authority,projection_epoch:'3',complete_through:now,observed_at:now},portfolio);await request(`${prefix}/account-closure/watermark`,{contract_version:2,operation:'PublishPublicProjectionWatermarkV1',source_authority:authority,family:'account_closure',projection_epoch:'3',complete_through:now,observed_at:now},character,true);}
  }catch(error){if(!stopped&&error.code!=='ECONNREFUSED')console.error('Local Game fixture exchange:',error.code||error.message);}finally{busy=false;}
 }
 const control=http.createServer((req,res)=>{
  if(req.method==='GET'&&req.url==='/fixtures/account-deletion/closure.json'){res.setHeader('content-type','application/json');res.end(fixtureBytes);return;}
  if(req.method==='POST'&&req.url==='/fault'){let body='';req.on('data',part=>{body+=part;if(body.length>512)req.destroy();});req.on('end',()=>{try{const input=JSON.parse(body);if(!['invalid_signature','missing_lcfa','hold','valid'].includes(input.mode))throw new Error('mode');faults.set(input.account_id,input.mode);res.end('{}');}catch{res.writeHead(400);res.end();}});return;}
  if(req.method==='GET'&&req.url==='/health'){res.end('ok');return;}
  res.writeHead(404);res.end();
 });
 return {publicKeyHex,keyId,controlURL:`http://127.0.0.1:${controlPort}`,async start(){await new Promise(resolve=>control.listen(controlPort,'127.0.0.1',resolve));timer=setInterval(tick,5000);await tick();},async stop(){stopped=true;clearInterval(timer);await new Promise(resolve=>control.close(resolve));}};
}
