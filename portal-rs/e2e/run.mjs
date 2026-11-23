import fs from 'node:fs';
import {execFileSync, spawn} from 'node:child_process';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
const [file,...args]=process.argv.slice(2);
if(!file||!args.length)throw new Error('Usage: node run.mjs /absolute/scratch/env.json test [Playwright arguments]');
const readJson = path => JSON.parse(fs.readFileSync(path, 'utf8'));
const env={...process.env,...readJson(file)};

const outboxCA = env.PORTAL_E2E_OUTBOX_ENV && readJson(env.PORTAL_E2E_OUTBOX_ENV).SSL_CERT_FILE;
const gameCA = env.PORTAL_E2E_GAME_DATA_REPORT && readJson(env.PORTAL_E2E_GAME_DATA_REPORT).caFile;
const fixtureCAs = new Set([env.SSL_CERT_FILE, outboxCA, gameCA].filter(Boolean));
if (fixtureCAs.size) {
  const browserHome = fs.mkdtempSync(join(tmpdir(), 'portal-e2e-browser-'));
  process.on('exit', () => fs.rmSync(browserHome, { recursive: true, force: true }));
  const nss = join(browserHome, '.pki/nssdb');
  fs.mkdirSync(nss, { recursive: true });
  const database = `sql:${nss}`;
  execFileSync('certutil', ['-N', '-d', database, '--empty-password']);
  for (const [index, ca] of [...fixtureCAs].entries()) {
    execFileSync('certutil', ['-A', '-d', database, '-n', `Portal E2E CA ${index}`, '-t', 'C,,', '-i', ca]);
  }
  const nodeCAs = new Set([...fixtureCAs, env.NODE_EXTRA_CA_CERTS].filter(Boolean));
  const bundle = join(browserHome, 'node-ca.pem');
  fs.writeFileSync(bundle, [...nodeCAs].map(ca => fs.readFileSync(ca, 'utf8')).join('\n'));
  env.NODE_EXTRA_CA_CERTS = bundle;
  env.PORTAL_E2E_CHROMIUM_HOME = browserHome;
}
const child=spawn(process.execPath,['node_modules/@playwright/test/cli.js',...args],{env,stdio:'inherit'});
child.on('exit',code=>process.exit(code??1));
