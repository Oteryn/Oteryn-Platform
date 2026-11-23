import { spawn } from 'node:child_process';
import { access, constants, mkdtemp, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { GamePublishers } from './game-publishers.mjs';
import { trustHarnessCa } from './trust.mjs';

async function trustGameData(reportFile) {
  const { report: { caFile } } = new GamePublishers(reportFile);
  const previous = process.env.NODE_EXTRA_CA_CERTS;
  const roots = previous ? await readFile(previous, 'utf8') : '';
  const gameCa = await readFile(caFile, 'utf8');
  const directory = await mkdtemp(path.join(os.tmpdir(), 'portal-e2e-game-ca-'));
  const bundle = path.join(directory, 'ca.pem');
  let releaseCa;
  try {
    await writeFile(bundle, `${roots}\n${gameCa}`, { mode: 0o600 });
    releaseCa = trustHarnessCa(caFile);
    // Playwright forks workers after setup. Their API clients need the same CA
    // as Chromium for the authenticated browser form requests.
    process.env.NODE_EXTRA_CA_CERTS = bundle;
  } catch (error) {
    await rm(directory, { recursive: true, force: true });
    throw error;
  }
  return async () => {
    if (previous === undefined) delete process.env.NODE_EXTRA_CA_CERTS;
    else process.env.NODE_EXTRA_CA_CERTS = previous;
    try {
      releaseCa();
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  };
}

export default async function setup(config) {
  const e2eDirectory = path.dirname(fileURLToPath(import.meta.url));
  // Nested stand-alone suites can inherit the browser config and use their own harness.
  if (config && await realpath(config.rootDir) !== e2eDirectory) return;
  if (process.env.PORTAL_E2E_GAME_DATA_REPORT) {
    return trustGameData(process.env.PORTAL_E2E_GAME_DATA_REPORT);
  }

  const binaryPath = process.env.PORTAL_E2E_BINARY;
  if (!binaryPath) throw new Error('Set PORTAL_E2E_BINARY to a prebuilt oteryn-portal for Game-data tests');
  const binary = path.resolve(fileURLToPath(new URL('../', import.meta.url)), binaryPath);
  await access(binary, constants.X_OK);

  const scratch = await mkdtemp(path.join(os.tmpdir(), 'portal-e2e-game-data-'));
  const output = path.join(scratch, 'harness');
  const reportFile = path.join(output, 'report.json');
  const child = spawn(process.execPath, ['harness.mjs', '--game-data'], {
    cwd: new URL('.', import.meta.url),
    env: {
      ...process.env,
      PORTAL_E2E_BINARY: binary,
      PORTAL_E2E_OUT: output,
      PORTAL_E2E_KEEP_DATABASE: '0',
      // The browser CI stack already owns the default public and internal ports.
      PORTAL_E2E_HTTP_PORT: '28080',
      PORTAL_E2E_INTERNAL_PORT: '28543',
      PORTAL_E2E_ADMISSION_PORT: '28091',
      PORTAL_E2E_GATEWAY_PORT: '28090',
    },
    stdio: ['ignore', 'pipe', 'inherit', 'ipc'],
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
  let releaseCa;
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
    releaseCa = await trustGameData(reportFile);
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
    try {
      const { code, signal } = await stop();
      if (code !== 0 || signal !== null) {
        const status = signal ? `signal ${signal}` : `exit code ${code}`;
        throw new Error(`Game-data harness shutdown failed (${status}). Harness logs are in ${path.join(output, 'logs')}`);
      }
      await rm(scratch, { recursive: true, force: true });
    } finally {
      await releaseCa();
    }
  };
}
