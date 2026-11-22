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
  const stop = async () => {
    child.kill('SIGTERM');
    // The harness first terminates its process groups and reconciles PostgreSQL.
    const deadline = setTimeout(() => child.kill('SIGKILL'), 105_000);
    try {
      return await stopped;
    } finally {
      clearTimeout(deadline);
    }
  };

  const lines = createInterface({ input: child.stdout });
  let readinessTimeout;
  try {
    await new Promise((resolve, reject) => {
      readinessTimeout = setTimeout(() => reject(new Error('Game-data harness readiness timed out')), 300_000);
      const onLine = line => {
        try {
          if (JSON.parse(line).status !== 'ready') throw new Error();
          resolve();
        } catch {
          reject(new Error('Game-data harness emitted an invalid readiness report'));
        }
      };
      child.once('error', reject);
      stopped.then(({ code, signal }) => reject(new Error(`Game-data harness exited before readiness (${signal ?? code})`)));
      lines.once('line', onLine);
      lines.once('close', () => reject(new Error('Game-data harness closed its readiness stream')));
    });
    await access(reportFile);
    process.env.PORTAL_E2E_GAME_DATA_REPORT = reportFile;
  } catch (error) {
    const { code, signal } = await stop();
    const status = signal ? `signal ${signal}` : `exit code ${code}`;
    throw new Error(`Game-data startup failed (${status}). Harness logs are in ${path.join(output, 'logs')}`, { cause: error });
  } finally {
    clearTimeout(readinessTimeout);
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
