import { randomBytes } from 'node:crypto';
import { spawn } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const [file, port = '18447', nativePort = '18444'] = process.argv.slice(2);
const binary = process.env.PORTAL_E2E_BINARY;
const validPort = value => /^\d+$/.test(value) && Number(value) >= 1024 && Number(value) <= 65535;
if (!file || !binary || !validPort(port) || !validPort(nativePort) || port === nativePort) {
  throw new Error('Provide the browser harness environment, portal binary, and distinct unused loopback ports');
}

const env = JSON.parse(readFileSync(file, 'utf8'));
const database = new URL(env.OTERYN_DATABASE_URL);
if (env.OTERYN_ENV !== 'testing' || !['localhost', '127.0.0.1', '[::1]'].includes(database.hostname)
    || !database.pathname.startsWith('/portal_e2e_') || env.OTERYN_BIND !== '127.0.0.1:18080'
    || env.OTERYN_INTERNAL_TLS_BIND === `127.0.0.1:${port}`) {
  throw new Error('Use the disposable browser harness and a different private listener port');
}

const child = spawn(binary, ['serve'], {
  cwd: fileURLToPath(new URL('../../', import.meta.url)),
  env: {
    ...process.env, ...env,
    OTERYN_INTERNAL_TLS_BIND: `127.0.0.1:${port}`,
    OTERYN_NATIVE_TLS_BIND: `127.0.0.1:${nativePort}`,
    OTERYN_NATIVE_TLS_CERT_FILE: env.OTERYN_INTERNAL_TLS_CERT_FILE,
    OTERYN_NATIVE_TLS_KEY_FILE: env.OTERYN_INTERNAL_TLS_KEY_FILE,
    OTERYN_ATLAS_BESTIARY_ENABLED: 'true',
    PORTAL_GROWTH_RANKINGS_ENABLED: 'true',
    PORTAL_PAYMENTS_PROVIDER_WEBHOOK_ENABLED: 'true',
    PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET: randomBytes(32).toString('hex'),
  },
  stdio: ['ignore', 'inherit', 'inherit'],
});
for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => child.kill(signal));
}
child.on('exit', code => { process.exitCode = code ?? 1; });
