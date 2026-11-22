import { spawn } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const [file, port = '18447'] = process.argv.slice(2);
const binary = process.env.PORTAL_E2E_BINARY;
if (!file || !binary || !/^\d+$/.test(port) || Number(port) < 1024 || Number(port) > 65535) {
  throw new Error('Provide the browser harness environment, portal binary, and an unused loopback port');
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
  env: { ...process.env, ...env, OTERYN_INTERNAL_TLS_BIND: `127.0.0.1:${port}` },
  stdio: ['ignore', 'inherit', 'inherit'],
});
for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => child.kill(signal));
}
child.on('exit', code => { process.exitCode = code ?? 1; });
