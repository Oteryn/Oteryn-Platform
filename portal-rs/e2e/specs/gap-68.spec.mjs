import { test, expect } from '@playwright/test';
import { execFile, spawn } from 'node:child_process';
import { once } from 'node:events';
import { chmod, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { promisify } from 'node:util';

const run = promisify(execFile);
const tokenKey = 'OTERYN_PLATFORM_SERVICE_TOKEN_FILE';

test('operator provisions a credential and starts the gateway', async ({ request }) => {
  const binary = process.env.PORTAL_E2E_BINARY;
  expect(Boolean(binary)).toBe(true);
  const directory = await mkdtemp(join(tmpdir(), 'portal-gap68-'));
  try {
    const file = join(directory, 'service-token');
    const token = 'synthetic-gateway-credential';
    await writeFile(file, token, { mode: 0o600 });
    const env = {
      [tokenKey]: file,
      OTERYN_PLATFORM_BASE_URL: 'http://127.0.0.1:1',
      GATEWAY_LISTEN_ADDR: '127.0.0.1:0',
      RUST_LOG: 'info',
    };

    await chmod(file, 0o640);
    const refused = await run(binary, ['gateway'], { env, timeout: 5000 }).then(
      () => false,
      error => error.code === 1 && error.stdout.includes(tokenKey) && !error.stdout.includes(token),
    );
    expect(refused).toBe(true);

    await chmod(file, 0o400);
    const reservation = createServer();
    reservation.listen(0, '127.0.0.1');
    await once(reservation, 'listening');
    const port = reservation.address().port;
    await new Promise((resolve, reject) => {
      reservation.close(error => error ? reject(error) : resolve());
    });
    const gateway = spawn(binary, ['gateway'], {
      env: { ...env, GATEWAY_LISTEN_ADDR: `127.0.0.1:${port}` },
      stdio: 'ignore',
    });
    const exited = once(gateway, 'close');
    try {
      await expect.poll(async () => {
        try {
          const response = await request.get(`http://127.0.0.1:${port}/health`, { timeout: 1000 });
          const status = response.ok() ? (await response.json()).status : undefined;
          await response.dispose();
          return status;
        } catch {
          return undefined;
        }
      }).toBe('ok');
      expect(gateway.exitCode).toBeNull();
    } finally {
      gateway.kill('SIGTERM');
      await exited;
    }
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
