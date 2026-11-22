import { spawn } from 'node:child_process';
import path from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';

export function startLocalOutbox(scratch, binary) {
  const output = path.resolve(scratch, 'outbox');
  const child = spawn(process.execPath, [fileURLToPath(new URL('../harness.mjs', import.meta.url)), '--outbox'], {
    env: {
      ...process.env,
      PORTAL_E2E_BINARY: path.resolve(binary),
      PORTAL_E2E_OUT: output,
      PORTAL_E2E_BROWSER_HTTPS_PORT: '',
      PORTAL_E2E_HTTP_PORT: '18180',
      PORTAL_E2E_INTERNAL_PORT: '18544',
      PORTAL_E2E_ADMISSION_PORT: '18191',
      PORTAL_E2E_GATEWAY_PORT: '18190',
    },
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  const stopped = new Promise(resolve => child.once('close', resolve));
  const ready = new Promise((resolve, reject) => {
    const report = createInterface({ input: child.stdout });
    report.once('line', line => {
      try {
        if (JSON.parse(line).status !== 'ready') throw new Error();
        resolve({
          PORTAL_E2E_OUTBOX_REPORT: path.join(output, 'report.json'),
          PORTAL_E2E_OUTBOX_ENV: path.join(output, 'secrets', 'env.json'),
        });
      } catch {
        reject(new Error('Invalid outbox harness readiness report'));
      } finally {
        report.close();
      }
    });
    child.once('error', error => reject(new Error(`Outbox harness spawn failed (${error.code})`)));
    child.once('close', (code, signal) => reject(new Error(`Outbox harness exited before readiness (${code ?? signal})`)));
  });
  return {
    ready,
    async close() {
      if (child.exitCode === null && child.signalCode === null) child.kill('SIGTERM');
      await stopped;
    },
  };
}
