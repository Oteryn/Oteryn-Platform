// Provide a native OAuth client and a sandbox package to the existing launcher.
// The wrapper delegates every invocation to the same real Portal binary.
import { randomBytes, createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

if (typeof process.execve !== 'function') throw new Error('Expiry requires Node 22.15 or later with process.execve');
const binary = process.env.PORTAL_E2E_BINARY;
if (!binary) throw new Error('Set PORTAL_E2E_BINARY to the built oteryn-portal binary');
const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'portal-expiry-'));
const tokenFile = path.join(scratch, 'admission-bearer');
const hashFile = path.join(scratch, 'admission-bearer.sha256');
const token = randomBytes(32).toString('base64url');
fs.writeFileSync(tokenFile, token, { mode: 0o600 });
fs.writeFileSync(hashFile, createHash('sha256').update(token).digest('hex'), { mode: 0o600 });
const wrapper = path.join(scratch, 'portal');
fs.writeFileSync(wrapper, `#!${process.execPath}
const { execve } = require('node:process');
const env = { ...process.env };
if (env.PORTAL_OAUTH_CONFIG) {
  const config = JSON.parse(env.PORTAL_OAUTH_CONFIG);
  config.clients.push({
    client_id: '019a0000-0000-7000-8000-0000000c11e7',
    redirect_uris: ['https://localhost:18443/__oauth/callback'],
  });
  env.PORTAL_OAUTH_CONFIG = JSON.stringify(config);
}
const binary = ${JSON.stringify(fs.realpathSync(binary))};
execve(binary, [binary, ...process.argv.slice(2)], env);
`, { mode: 0o700 });

const child = spawn(process.execPath, [fileURLToPath(new URL('../local.mjs', import.meta.url))], {
  env: {
    ...process.env,
    PORTAL_E2E_SCRATCH: scratch,
    PORTAL_E2E_BINARY: wrapper,
    PORTAL_E2E_EXPIRY_ADMISSION_BEARER_FILE: tokenFile,
    PORTAL_GATEWAY_TOKEN_SHA256_FILE: hashFile,
    PORTAL_NATIVE_ADMISSION_BIND: '127.0.0.1:8091',
    PORTAL_ACCOUNT_CLOSURE_FINAL_DISPOSITION_POLICY: 'hold',
    PORTAL_COIN_PACKAGES: JSON.stringify([{
      id: '019a0000-0000-7000-8000-000000003001',
      title: 'Expiry test coins',
      coins: 1234,
      price: { minor_units: 250, currency: 'PLN', exponent: 2 },
      valid_for_seconds: 60,
    }]),
  },
  stdio: 'inherit',
});
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => child.kill(signal));
child.once('exit', (code, signal) => { process.exitCode = code ?? (signal ? 1 : 0); });
