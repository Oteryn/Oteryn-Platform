import fs from 'node:fs';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {trustHarnessCa} from './trust.mjs';
const [file,...args]=process.argv.slice(2);
if(!file||!args.length)throw new Error('Usage: node run.mjs /absolute/scratch/env.json test [Playwright arguments]');
const env={...process.env,...JSON.parse(fs.readFileSync(file,'utf8'))};
const releases=[];
let bundle;
try {
 if(!args.includes('--list')){
  const files=[env.SSL_CERT_FILE];
  if(env.PORTAL_E2E_OUTBOX_ENV){
   files.push(JSON.parse(fs.readFileSync(env.PORTAL_E2E_OUTBOX_ENV,'utf8')).SSL_CERT_FILE);
  }
  const certificates=[...new Set(files.filter(Boolean))];
  for(const ca of certificates)releases.push(trustHarnessCa(ca));
  if(certificates.length){
   bundle=fs.mkdtempSync(join(tmpdir(),'portal-e2e-ca-'));
   env.NODE_EXTRA_CA_CERTS=join(bundle,'ca.pem');
   fs.writeFileSync(env.NODE_EXTRA_CA_CERTS,certificates.map(ca=>fs.readFileSync(ca,'utf8')).join('\n'));
  }
 }
 const child=spawn(process.execPath,['node_modules/@playwright/test/cli.js',...args],{env,stdio:'inherit'});
 const interrupt=()=>child.kill('SIGINT');
 const terminate=()=>child.kill('SIGTERM');
 process.once('SIGINT',interrupt);
 process.once('SIGTERM',terminate);
 try {
  const [code]=await once(child,'exit');
  process.exitCode=code??1;
 } finally {
  process.off('SIGINT',interrupt);
  process.off('SIGTERM',terminate);
 }
} finally {
 for(const release of releases.reverse())release();
 if(bundle)fs.rmSync(bundle,{recursive:true,force:true});
}
