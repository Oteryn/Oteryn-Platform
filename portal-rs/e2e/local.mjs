// Disposable local services. Run only against a dedicated local PostgreSQL instance.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';
import https from 'node:https';
import http from 'node:http';
import dns from 'node:dns';
import { spawn, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import pg from 'pg';
import { SMTPServer } from 'smtp-server';
dns.setDefaultResultOrder('ipv4first');
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const scratch=process.env.PORTAL_E2E_SCRATCH || fs.mkdtempSync(path.join(os.tmpdir(),'portal-e2e-'));
fs.mkdirSync(scratch,{recursive:true,mode:0o700});
const adminURL=process.env.PORTAL_E2E_ADMIN_DATABASE_URL;
if(!adminURL) throw new Error('Set PORTAL_E2E_ADMIN_DATABASE_URL to a throwaway local PostgreSQL administrator URL');
if(!['localhost','127.0.0.1','[::1]'].includes(new URL(adminURL).hostname)) throw new Error('Local PostgreSQL only');
const dbName=`portal_e2e_${crypto.randomBytes(6).toString('hex')}`;
const testName=`${dbName}_tests`;
const admin=new pg.Client({connectionString:adminURL});await admin.connect();
for(const name of [dbName,testName]) await admin.query(`CREATE DATABASE "${name}"`);
const databaseURL=name=>{const url=new URL(adminURL);url.pathname=`/${name}`;return url.toString();};
const env={...process.env};
function walk(directory){return fs.readdirSync(directory,{withFileTypes:true}).flatMap(e=>e.isDirectory()?walk(path.join(directory,e.name)):[path.join(directory,e.name)]);}
for(const file of [...walk(path.join(root,'crates')),...walk(path.join(root,'bin'))].filter(p=>p.endsWith('.rs'))){
 for(const [,key] of fs.readFileSync(file,'utf8').matchAll(/"((?:OTERYN|PORTAL|GAME_AUTH)_[A-Z0-9_]+_ENABLED)"/g)) env[key]='true';
}
for(const key of ['OTERYN_ACCOUNT_LOGIN','OTERYN_ACCOUNT_MFA','OTERYN_ACCOUNT_PASSKEYS','OTERYN_ACCOUNT_PASSWORD_RESET','OTERYN_ACCOUNT_SESSIONS','OTERYN_ACCOUNT_CREDENTIALS','OTERYN_ACCOUNT_EMAIL_CHANGE','OTERYN_ADMIN_PANEL','OTERYN_ADMIN_CONSOLE','OTERYN_ADMIN_PLAYERS','OTERYN_ADMIN_PLAYER_CHARACTERS','OTERYN_ADMIN_COMMERCE','OTERYN_ADMIN_CONTROL','OTERYN_WORLD_ADMIN','OTERYN_BAZAAR_MODERATION','OTERYN_BAZAAR_PROFILES','OTERYN_BAZAAR_PROMOTIONS','OTERYN_SUPPORT_TICKETS','OTERYN_PLAYERDATA_HUNTS_READ','OTERYN_PLAYERDATA_PLACES_AGGREGATE']) env[key]='true';
Object.assign(env,{
 OTERYN_ENV:'testing',APP_ENV:'testing',PANEL_PREMIUM_PAYMENTS:'true',OTERYN_BIND:'127.0.0.1:18080',OTERYN_PUBLIC_ORIGIN:'https://localhost:18443',OTERYN_ADMIN_HOST:'localhost:18443',
 OTERYN_STATIC_DIR:path.join(root,'static'),OTERYN_DATABASE_URL:databaseURL(dbName),PORTAL_E2E_DATABASE_URL:databaseURL(dbName),PORTAL_TEST_DATABASE_URL:databaseURL(testName),
 OTERYN_SMTP_HOST:'localhost',OTERYN_SMTP_USERNAME:'local',OTERYN_SMTP_PASSWORD:crypto.randomBytes(24).toString('hex'),OTERYN_MAIL_FROM:'portal@local.test',OTERYN_SMTP_FROM:'portal@local.test',
 OTERYN_WEBAUTHN_RP_ID:'localhost',OTERYN_MFA_KEY:crypto.randomBytes(32).toString('hex'),OTERYN_RESET_DELIVERY_KEY:crypto.randomBytes(32).toString('hex'),
 OTERYN_NOTIFY_UNSUBSCRIBE_KEY:crypto.randomBytes(32).toString('hex'),OTERYN_NOTIFY_UNSUBSCRIBE_URL:'https://localhost:18443/en/unsubscribe',
 OTERYN_ACTIVE_CHARACTER_LIMIT:'10',OTERYN_TOTAL_CHARACTER_LIMIT:'20',OTERYN_CHARACTER_COMMAND_SOURCE_AUTHORITY:'e2e-local',OTERYN_LCFA_SOURCE_AUTHORITY:'e2e-local',OTERYN_ACCOUNT_CHARACTERS_AUTHORITY:'e2e-local',
 PORTAL_PUBLIC_PROJECTIONS_AUTHORITY:'e2e-local',PORTAL_PUBLIC_PROJECTIONS_SAN:'e2e-local',PORTAL_WORLD_DAILY_WORLDS:'local',PORTAL_ATLAS_DELIVERY_ENABLED:'1',
 OTERYN_ATLAS_DIRECTORY:path.join(scratch,'atlas'),SSL_CERT_FILE:path.join(scratch,'ca.pem'),
 PORTAL_ADMIN_CONTROL_AGENTS_JSON:JSON.stringify([{cn:'oteryn-admin-agent/e2e-local',identity:{san:'spiffe://local.test/admin/e2e'}}]),
});
fs.mkdirSync(env.OTERYN_ATLAS_DIRECTORY,{recursive:true});
function command(program,args,options={}){const result=spawnSync(program,args,{cwd:root,env,stdio:'inherit',...options});if(result.status!==0)throw new Error(`${program} failed`);}
function openssl(args){command('openssl',args,{stdio:'ignore'});}
openssl(['req','-x509','-newkey','rsa:2048','-nodes','-keyout',`${scratch}/ca-key.pem`,'-out',`${scratch}/ca.pem`,'-days','2','-subj','/CN=PortalLocalCA','-addext','basicConstraints=critical,CA:TRUE']);
openssl(['req','-newkey','rsa:2048','-nodes','-keyout',`${scratch}/key.pem`,'-out',`${scratch}/leaf.csr`,'-subj','/CN=localhost']);
fs.writeFileSync(`${scratch}/extensions`,'subjectAltName=DNS:localhost,IP:127.0.0.1\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n');
openssl(['x509','-req','-in',`${scratch}/leaf.csr`,'-CA',`${scratch}/ca.pem`,'-CAkey',`${scratch}/ca-key.pem`,'-CAcreateserial','-out',`${scratch}/cert.pem`,'-days','2','-extfile',`${scratch}/extensions`]);
fs.writeFileSync(`${scratch}/env.json`,JSON.stringify(Object.fromEntries(Object.entries(env).filter(([key])=>(/^(OTERYN|PORTAL|GAME_AUTH)_/.test(key)&&key!=='PORTAL_E2E_ADMIN_DATABASE_URL')||['APP_ENV','SSL_CERT_FILE','PANEL_PREMIUM_PAYMENTS'].includes(key)))),{mode:0o600});
command('cargo',['build','--locked','-p','oteryn-portal']);
const binary=process.env.PORTAL_E2E_BINARY||path.join(process.env.CARGO_TARGET_DIR||path.join(root,'target'),'debug/oteryn-portal');
command(binary,['migrate']);
const tls={key:fs.readFileSync(`${scratch}/key.pem`),cert:fs.readFileSync(`${scratch}/cert.pem`)};
const messages=[];
const smtp=new SMTPServer({...tls,secure:true,onAuth(auth,session,callback){callback(null,{user:'local'});},onData(stream,session,callback){let raw='';stream.on('data',chunk=>raw+=chunk);stream.on('end',()=>{messages.push({to:session.envelope.rcptTo.map(r=>r.address),raw});callback();});}});
smtp.on('error',error=>console.error('Local SMTP error:',error.code));
smtp.listen(465,'127.0.0.1');
const proxy=https.createServer(tls,(req,res)=>{
 if(req.url==='/__mail'){res.setHeader('content-type','application/json');res.end(JSON.stringify(messages));return;}
 const upstream=http.request({hostname:'127.0.0.1',port:18080,path:req.url,method:req.method,headers:req.headers},response=>{res.writeHead(response.statusCode,response.headers);response.pipe(res);});
 upstream.on('error',()=>{res.writeHead(502);res.end();});req.pipe(upstream);
});
proxy.listen(18443,'127.0.0.1');
const children=['serve','worker'].map(role=>spawn(binary,[role],{cwd:root,env,stdio:['ignore',fs.openSync(`${scratch}/${role}.log`,'a'),fs.openSync(`${scratch}/${role}.log`,'a')]}));
console.log(`Services starting. Scratch: ${scratch}\nRun: node run.mjs ${scratch}/env.json test\nStop with Ctrl-C to stop services and drop the two task databases.`);
let closing=false;
async function close(){if(closing)return;closing=true;for(const child of children)child.kill('SIGINT');await Promise.all(children.map(child=>child.exitCode!==null?Promise.resolve():new Promise(resolve=>child.once('exit',resolve))));smtp.close();proxy.close();for(const name of [dbName,testName])await admin.query(`DROP DATABASE "${name}" WITH (FORCE)`);await admin.end();process.exit(0);}
process.on('SIGINT',close);process.on('SIGTERM',close);
