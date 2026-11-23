import { spawn } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { localPostgresUrl } from '../postgres-url.mjs';
import { fixtureDatabaseClaim, removeDatabase } from './harness-database.mjs';
import { withDeadline } from './harness-deadline.mjs';

const harness = fileURLToPath(new URL('../harness.mjs', import.meta.url));

export function spawnFixtureHarness(output, adminURL, env, args = []) {
  return spawn(process.execPath, [harness, '--fixture-owned', ...args], {
    env: {
      ...env,
      PORTAL_E2E_ADMIN_DATABASE_URL: localPostgresUrl(adminURL).href,
      PORTAL_E2E_OUT: output,
      PORTAL_E2E_KEEP_DATABASE: '0',
      PORTAL_E2E_SEED_PLAN: '',
    },
    detached: true,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

export async function stopFixtureHarness(child, exited, output, adminURL, graceMs = 34_000) {
  let failure;
  try {
    child.kill('SIGTERM');
    const [code, signal] = await withDeadline(exited, graceMs, 'harness shutdown timed out');
    if (code === 0) return;
    failure = new Error(`harness exited with ${code ?? signal}`);
  } catch (error) {
    failure = error;
  }

  // The harness and every subprocess inherited the fixture's process group.
  if (child.pid !== undefined) {
    try {
      process.kill(-child.pid, 'SIGKILL');
    } catch (error) {
      if (error.code !== 'ESRCH') throw error;
    }
    await withDeadline(exited, 2_000, 'harness did not exit after SIGKILL');
  }
  // The claim exists only after the harness checked the name under the same
  // lock used for DROP. CREATE might have committed without sending a reply.
  const database = await readFile(join(output, fixtureDatabaseClaim), 'utf8').catch(error => {
    if (error.code !== 'ENOENT') throw error;
  });
  if (database !== undefined) await removeDatabase(localPostgresUrl(adminURL), database);
  throw failure;
}
