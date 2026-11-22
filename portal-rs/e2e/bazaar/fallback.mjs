import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createInterface } from 'node:readline';

export async function withFallback(adminURL, run) {
  const scratch = await mkdtemp(join(tmpdir(), 'portal-e2e-bazaar-'));
  const output = join(scratch, 'run');
  const origin = 'http://127.0.0.1:18082';
  const child = spawn(process.execPath, ['harness.mjs'], {
    cwd: new URL('../', import.meta.url),
    env: {
      ...process.env,
      PORTAL_E2E_ADMIN_DATABASE_URL: adminURL,
      PORTAL_E2E_OUT: output,
      PORTAL_E2E_KEEP_DATABASE: '0',
      PORTAL_E2E_SEED_PLAN: '',
      PORTAL_E2E_HTTP_PORT: '18082',
      OTERYN_PUBLIC_ORIGIN: origin,
      PORTAL_GAME_RESTORE_NOTICE_ENABLED: 'false',
    },
    stdio: ['ignore', 'pipe', 'ignore'],
  });
  const exited = new Promise(resolve => child.once('close', resolve));
  const outputLines = createInterface({ input: child.stdout });
  try {
    const [line] = await Promise.race([
      once(outputLines, 'line', { signal: AbortSignal.timeout(60_000) }),
      once(child, 'exit').then(([code]) => { throw new Error(`Bazaar fallback harness exited before readiness (${code})`); }),
    ]);
    if (JSON.parse(line).status !== 'ready') throw new Error('Bazaar fallback harness did not report readiness');
    const env = JSON.parse(await readFile(join(output, 'secrets', 'env.json'), 'utf8'));
    await run(origin, env.OTERYN_DATABASE_URL);
  } finally {
    outputLines.close();
    child.kill('SIGTERM');
    await exited;
    await rm(scratch, { recursive: true, force: true });
  }
}
