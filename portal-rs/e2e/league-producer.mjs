// TEST/E2E ONLY. Generates disposable keys externally; never imported by production.
// Replays the pinned canonical JSON over TLS1.3/mTLS, exactly as Game does.
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
 leaf('league-platform','serverAuth');leaf('league-game','clientAuth');leaf('league-other','clientAuth');
 openssl(['pkcs8','-topk8','-nocrypt','-in',`${directory}/league-platform.key`,'-outform','DER','-out',`${directory}/league-platform-key.der`]);
 openssl(['x509','-in',`${scratch}/ca.pem`,'-outform','DER','-out',`${directory}/ca.der`]);
 const hash=b=>crypto.createHash('sha256').update(b).digest('hex');
 const epoch='3',generation='1',authority='oteryn:game:primary';
 fs.writeFileSync(`${directory}/recovery.json`,JSON.stringify({source_authority:authority,projection_epoch:epoch,recovery_generation:generation}),{mode:0o600});
 const audience='https://localhost:18444';
 Object.assign(env,{
  OTERYN_LEAGUE_ENABLED:'true',OTERYN_LEAGUE_BASE_URL:audience,OTERYN_LEAGUE_BIND:'127.0.0.1:18444',OTERYN_LEAGUE_SOURCE_AUTHORITY:authority,
  OTERYN_LEAGUE_CLIENT_CA_DER:`${directory}/ca.der`,OTERYN_LEAGUE_CERTIFICATE_DER:`${directory}/league-platform.der`,OTERYN_LEAGUE_PRIVATE_KEY_DER:`${directory}/league-platform-key.der`,
  OTERYN_LEAGUE_CLIENT_SPKI_SHA256:'sha256:'+hash(new crypto.X509Certificate(fs.readFileSync(`${directory}/league-game.der`)).publicKey.export({type:'spki',format:'der'})),
  OTERYN_LEAGUE_RECOVERY_FILE:`${directory}/recovery.json`,PORTAL_E2E_LEAGUE_CONTROL:'http://127.0.0.1:18445',
 });
 const fixtures=JSON.parse(fs.readFileSync(path.join(root,'crates/minigames/src/league/fixtures/snapshots.json')));
 const records=fixtures.vectors.map(v=>{
  if(hash(v.wire)!==v.sha256)throw Error('pinned wire hash mismatch');
  const s=JSON.parse(v.wire);
  return {name:v.name,path:`/internal/v1/game-public/${s.season?'guild-season':'world-event-outcome'}`,body:v.wire};
 });
 const options={ca:fs.readFileSync(`${scratch}/ca.pem`),key:fs.readFileSync(`${directory}/league-game.key`),cert:fs.readFileSync(`${directory}/league-game.pem`),minVersion:'TLSv1.3',maxVersion:'TLSv1.3',ALPNProtocols:['http/1.1'],agent:false};
 let mode='good',published=false,closed=false,refusalStatus=null,task;
 const send=(r,bad=false)=>new Promise(resolve=>{
  const identity=bad?{key:fs.readFileSync(`${directory}/league-other.key`),cert:fs.readFileSync(`${directory}/league-other.pem`)}:{};
  const req=https.request(`${audience}${r.path}`,{...options,...identity,method:'POST',headers:{'content-type':'application/json','content-length':Buffer.byteLength(r.body)}},res=>{
   let size=0;res.on('data',b=>{size+=b.length;});res.on('end',()=>resolve(res.headers['cache-control']==='no-store'&&size<=256&&(res.statusCode===200||size===0)?res.statusCode:500));
  });
  req.setTimeout(5000,()=>req.destroy());req.on('error',()=>resolve(503));req.end(r.body);
 });
 const pause=ms=>new Promise(r=>setTimeout(r,ms));
 const retry=async r=>{let backoff=1000;while(!closed){const status=await send(r);if(status===200)return; if([400,401,409].includes(status))throw Error(`league producer refused ${r.name||r.path}: ${status}`);await pause(backoff);backoff=Math.min(backoff*2,10000);}};
 const controls=http.createServer(async(req,res)=>{
  res.setHeader('content-type','application/json');
  if(req.method==='POST'){
   if(req.url==='/pause')mode='paused';else if(req.url==='/resume')mode='good';
   else if(req.url==='/wrong-identity'){refusalStatus=await send(records[0],true);mode='paused';}
   else{res.writeHead(404);res.end('{}');return;}
  }
  res.end(JSON.stringify({mode,published,refusalStatus}));
 });
 await new Promise(r=>controls.listen(18445,'127.0.0.1',r));
 return {
  start(){task=(async()=>{const registry=new pg.Client({connectionString:env.OTERYN_DATABASE_URL});await registry.connect();try{await registry.query('INSERT INTO world_status(id,name) VALUES($1,$2) ON CONFLICT DO NOTHING',[JSON.parse(records[0].body).world_id,'Synthetic fixture world']);}finally{await registry.end();}for(const r of records)await retry(r);published=true;while(!closed){if(mode==='good'){const seconds=String(Math.floor(Date.now()/1000));for(const family of ['world_event_outcome','guild_season'])await retry({path:'/internal/v1/game-public/league-watermark',body:JSON.stringify({contract_version:1,operation:'PublishLeagueWatermarkV1',source_authority:authority,family,projection_epoch:epoch,complete_through:seconds,observed_at:seconds})});}await pause(2000);}})().catch(e=>{console.error(e);closed=true;});},
  async close(){closed=true;await task;await new Promise(r=>controls.close(r));},
 };
}
