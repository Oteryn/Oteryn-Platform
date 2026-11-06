// TEST/E2E ONLY. Generates disposable keys externally; never imported by production.
// The complete pinned snapshots.json remains unchanged; native ANL envelopes and
// transport signatures below are synthetic test authority, not Game qualification.
import fs from 'node:fs';
import crypto from 'node:crypto';
import https from 'node:https';
import http from 'node:http';
import path from 'node:path';
import pg from 'pg';
export async function leagueProducer({root,scratch,env,openssl}) {
 const directory=path.join(scratch,'league');fs.mkdirSync(directory,{mode:0o700});
 const leaf=(name,usage)=>{
  openssl(['req','-newkey','rsa:2048','-nodes','-keyout',`${directory}/${name}.key`,'-out',`${directory}/${name}.csr`,'-subj',`/CN=${name}`]);
  fs.writeFileSync(`${directory}/${name}.ext`,`subjectAltName=DNS:localhost,IP:127.0.0.1\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=${usage}\n`);
  openssl(['x509','-req','-in',`${directory}/${name}.csr`,'-CA',`${scratch}/ca.pem`,'-CAkey',`${scratch}/ca-key.pem`,'-CAcreateserial','-out',`${directory}/${name}.pem`,'-days','2','-extfile',`${directory}/${name}.ext`]);
  openssl(['x509','-in',`${directory}/${name}.pem`,'-outform','DER','-out',`${directory}/${name}.der`]);
 };
 leaf('league-platform','serverAuth');leaf('league-game','clientAuth');
 openssl(['pkcs8','-topk8','-nocrypt','-in',`${directory}/league-platform.key`,'-outform','DER','-out',`${directory}/league-platform-key.der`]);
 openssl(['x509','-in',`${scratch}/ca.pem`,'-outform','DER','-out',`${directory}/ca.der`]);
 const keys=crypto.generateKeyPairSync('ed25519');
 const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
 const uuid=()=>{let b=crypto.randomBytes(16);let t=BigInt(Date.now());for(let i=5;i>=0;i--){b[i]=Number(t&255n);t>>=8n;}b[6]=(b[6]&15)|112;b[8]=(b[8]&63)|128;const h=b.toString('hex');return `${h.slice(0,8)}-${h.slice(8,12)}-${h.slice(12,16)}-${h.slice(16,20)}-${h.slice(20)}`;};
 const id=s=>Buffer.from(s.replaceAll('-',''),'hex');
 const vint=n=>{n=BigInt(n);const b=[];do{let v=Number(n&127n);n>>=7n;b.push(v|(n?128:0));}while(n);return Buffer.from(b);};
 const field=(tag,data,number=false)=>number?Buffer.concat([vint(tag*8),vint(data)]):Buffer.concat([vint(tag*8+2),vint(Buffer.byteLength(data)),Buffer.from(data)]);
 const proto=fields=>Buffer.concat(fields);
 const epoch='3',generation='1',authority='oteryn:game:primary';
 fs.writeFileSync(`${directory}/recovery.json`,JSON.stringify({source_authority:authority,projection_epoch:epoch,recovery_generation:generation}),{mode:0o600});
 const audience='https://localhost:18444';
 Object.assign(env,{
  OTERYN_LEAGUE_ENABLED:'true',OTERYN_LEAGUE_BASE_URL:audience,OTERYN_LEAGUE_BIND:'127.0.0.1:18444',OTERYN_LEAGUE_SOURCE_AUTHORITY:authority,
  OTERYN_LEAGUE_CLIENT_CA_DER:`${directory}/ca.der`,OTERYN_LEAGUE_CERTIFICATE_DER:`${directory}/league-platform.der`,OTERYN_LEAGUE_PRIVATE_KEY_DER:`${directory}/league-platform-key.der`,
  OTERYN_LEAGUE_PEER_CERTIFICATE_SHA256:hash(fs.readFileSync(`${directory}/league-game.der`)),
  OTERYN_LEAGUE_PUBLIC_KEYS:JSON.stringify({'e2e-disposable':keys.publicKey.export({type:'spki',format:'der'}).subarray(-32).toString('hex')}),
  OTERYN_LEAGUE_RECOVERY_FILE:`${directory}/recovery.json`,PORTAL_E2E_LEAGUE_CONTROL:'http://127.0.0.1:18445',
 });
 const fixtures=JSON.parse(fs.readFileSync(path.join(root,'crates/minigames/src/league/fixtures/snapshots.json')));
 const created=Math.floor(Date.now()/1000);const records=[];
 for(const v of fixtures.vectors){
  if(hash(v.wire)!==v.sha256)throw Error('pinned wire hash mismatch');
  const s=JSON.parse(v.wire),season=Boolean(s.season),r=(s.event||s.season)[0],raw=Buffer.from(JSON.stringify(r)),digest=Buffer.from(hash(raw),'hex');
  const envelopeID=uuid(),type=season?11:10,subject=s.event_id||s.season_id;
  const pending=['SCHEDULED','LOCKED'].includes(r.state);
  const payload=season?proto([field(1,digest),field(2,1,true),field(3,id(subject))]):proto([field(1,raw),field(2,digest),field(3,Buffer.from(r.evidence_sha256[0]||'00'.repeat(32),'hex')),field(4,1,true),field(5,id(subject))]);
  const envelope=proto([field(1,1,true),field(2,id(envelopeID)),field(3,type,true),field(4,1,true),field(5,2,true),field(6,3,true),field(7,'PLAT3_AUTHORITY_AUDIT_RETENTION_V1'),field(8,created*1000,true),field(9,id(s.world_id)),field(18,proto([field(1,id(envelopeID)),field(2,1,true),field(3,1,true)])),field(25,'e2e-only'),field(26,payload),field(27,Buffer.from(hash(payload),'hex'))]);
  const final=r.state==='CLOSED'||r.state==='FROZEN'||(season&&r.state==='VOID');
  records.push({name:v.name,path:`/internal/v1/game-public/${season?'guild-season':'world-event-outcome'}`,body:v.wire,envelope:pending?'':envelope.toString('base64'),finalized:final?[String(created)]:[],request:envelopeID,
   subject:{family:season?'guild_season':'world_event_outcome',world_id:s.world_id,subject_id:subject,projection_revision:s.projection_revision,record_sha256:hash(raw),withdrawn:false}});
 }
 const latest=new Map();for(const r of records)latest.set(`${r.subject.family}/${r.subject.world_id}/${r.subject.subject_id}`,r.subject);
 const subjects=[...latest.values()].sort((a,b)=>`${a.family}/${a.world_id}/${a.subject_id}`.localeCompare(`${b.family}/${b.world_id}/${b.subject_id}`,'en'));
 const cut={path:'/internal/v1/game-public/league-cut',body:JSON.stringify({contract_version:1,operation:'PublishLeagueCutV1',source_authority:authority,projection_epoch:epoch,recovery_generation:generation,subjects}),request:uuid()};
 const options={ca:fs.readFileSync(`${scratch}/ca.pem`),key:fs.readFileSync(`${directory}/league-game.key`),cert:fs.readFileSync(`${directory}/league-game.pem`),minVersion:'TLSv1.3',maxVersion:'TLSv1.3',ALPNProtocols:['http/1.1'],agent:false};
 let mode='good',published=false,closed=false,refusalStatus=null,task;
 const send=(r,bad=false)=>new Promise(resolve=>{
  const meta=JSON.stringify({contract_version:1,source_authority:authority,projection_epoch:epoch,recovery_generation:generation,key_id:'e2e-disposable',request_id:r.request,envelope:r.envelope||'',finalized_at:r.finalized||[]});
  const encoded=Buffer.from(meta).toString('base64');
  const input=Buffer.from(`oteryn-platform-league-admission-v1\n${audience}\nPOST\n${r.path}\n${encoded}\n${hash(r.body)}`);
  const signature=crypto.sign(null,input,keys.privateKey);if(bad)signature[0]^=1;
  const req=https.request(`${audience}${r.path}`,{...options,method:'POST',headers:{'content-type':'application/json','content-length':Buffer.byteLength(r.body),'x-oteryn-league-admission':encoded,'x-oteryn-league-signature':signature.toString('hex')}},res=>{res.resume();res.on('end',()=>resolve(res.statusCode));});
  req.setTimeout(5000,()=>req.destroy());req.on('error',()=>resolve(503));req.end(r.body);
 });
 const pause=ms=>new Promise(r=>setTimeout(r,ms));
 const retry=async r=>{while(!closed){const status=await send(r);if(status===200)return; if([400,401,409].includes(status))throw Error(`league producer refused ${r.name||r.path}: ${status}`);await pause(500);}};
 const controls=http.createServer(async(req,res)=>{
  res.setHeader('content-type','application/json');
  if(req.method==='POST'){
   if(req.url==='/pause')mode='paused';else if(req.url==='/resume')mode='good';
   else if(req.url==='/bad-signature'){refusalStatus=await send({...cut,request:uuid()},true);mode='paused';}
   else{res.writeHead(404);res.end('{}');return;}
  }
  res.end(JSON.stringify({mode,published,refusalStatus}));
 });
 await new Promise(r=>controls.listen(18445,'127.0.0.1',r));
 return {
  start(){task=(async()=>{const registry=new pg.Client({connectionString:env.OTERYN_DATABASE_URL});await registry.connect();try{await registry.query('INSERT INTO world_status(id,name) VALUES($1,$2) ON CONFLICT DO NOTHING',[subjects[0].world_id,'Synthetic fixture world']);}finally{await registry.end();}await retry(cut);for(const r of records)await retry(r);published=true;while(!closed){if(mode==='good'){const seconds=String(Math.floor(Date.now()/1000));for(const family of ['world_event_outcome','guild_season'])await retry({path:'/internal/v1/game-public/league-watermark',request:uuid(),body:JSON.stringify({contract_version:1,operation:'PublishLeagueWatermarkV1',source_authority:authority,family,projection_epoch:epoch,complete_through:seconds,observed_at:seconds})});}await pause(2000);}})().catch(e=>{console.error(e);closed=true;});},
  async close(){closed=true;await task;await new Promise(r=>controls.close(r));},
 };
}
