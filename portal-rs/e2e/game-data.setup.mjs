import { spawn } from 'node:child_process';
import { access, mkdtemp, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createInterface } from 'node:readline';

export default async function setup() {
  if (process.env.PORTAL_E2E_GAME_DATA_REPORT) {
    await access(process.env.PORTAL_E2E_GAME_DATA_REPORT);
    return;
  }

  const scratch = await mkdtemp(path.join(os.tmpdir(), 'portal-e2e-game-data-'));
  const output = path.join(scratch, 'harness');
  const reportFile = path.join(output, 'report.json');
  const child = spawn(process.execPath, ['harness.mjs', '--game-data'], {
    cwd: new URL('.', import.meta.url),
    env: {
      ...process.env,
      PORTAL_E2E_OUT: output,
      PORTAL_E2E_KEEP_DATABASE: '0',
      // The browser CI stack already owns the default public and internal ports.
      PORTAL_E2E_HTTP_PORT: '28080',
      PORTAL_E2E_INTERNAL_PORT: '28543',
      PORTAL_E2E_ADMISSION_PORT: '28091',
      PORTAL_E2E_GATEWAY_PORT: '28090',
    },
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  const stopped = new Promise(resolve => child.once('close', (code, signal) => resolve({ code, signal })));
  const stop = () => {
    child.kill('SIGTERM');
    return stopped;
  };

  const lines = createInterface({ input: child.stdout });
  try {
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('Game-data harness readiness timed out')), 300_000);
      const onExit = code => reject(new Error(`Game-data harness exited before readiness (${code})`));
      const onLine = line => {
        try {
          if (JSON.parse(line).status === 'ready') resolve();
        } catch {
          reject(new Error('Game-data harness emitted an invalid readiness report'));
        }
      };
      child.once('error', reject);
      child.once('exit', onExit);
      lines.once('line', onLine);
      stopped.then(() => clearTimeout(timer));
      lines.once('close', () => clearTimeout(timer));
    });
    await access(reportFile);
    process.env.PORTAL_E2E_GAME_DATA_REPORT = reportFile;
  } catch (error) {
    await stop();
    throw new Error(`Game-data startup failed. Harness logs are in ${path.join(output, 'logs')}`, { cause: error });
  } finally {
    lines.close();
  }

  return async () => {
    delete process.env.PORTAL_E2E_GAME_DATA_REPORT;
    const { code, signal } = await stop();
    if (code !== 0 || signal !== null) {
      const status = signal ? `signal ${signal}` : `exit code ${code}`;
      throw new Error(`Game-data harness shutdown failed (${status}). Harness logs are in ${path.join(output, 'logs')}`);
    }
    await rm(scratch, { recursive: true, force: true });
  };
}
