// The harness owns this supervisor. Playwright owns the worker's SIGTERM so
// it can observe the drain before it asks the harness to tear down PostgreSQL.
import { spawn } from 'node:child_process';
import { createHmac } from 'node:crypto';
import { once } from 'node:events';
import { writeFileSync } from 'node:fs';
import path from 'node:path';
import { createServer } from 'node:net';
import { createInterface } from 'node:readline';

const [binary, ...args] = process.argv.slice(2);
const scratch = path.dirname(path.dirname(process.env.OTERYN_INTERNAL_TLS_KEY_FILE));
const env = {
  ...process.env,
  RUST_LOG: 'info',
  OTERYN_ACCOUNT_SESSIONS: 'true',
  OTERYN_ACCOUNT_RECOVERY_KEYS: 'true',
  OTERYN_ACCOUNT_EMAIL_CHANGE: 'true',
  OTERYN_ACCOUNT_MFA: 'true',
  OTERYN_ACCOUNT_PASSKEYS: 'true',
  OTERYN_SUPPORT_TICKETS: 'true',
  OTERYN_NOTIFY_UNSUBSCRIBE_KEY: createHmac('sha256', process.env.OTERYN_MFA_KEY).update('worker-jobs unsubscribe').digest('hex'),
  OTERYN_NOTIFY_UNSUBSCRIBE_URL: new URL('/en/unsubscribe', process.env.OTERYN_PUBLIC_ORIGIN).href,
  OTERYN_ADMIN_PANEL: 'true',
  OTERYN_ADMIN_HOST: new URL(process.env.OTERYN_PUBLIC_ORIGIN).host,
  OTERYN_ADMIN_CONSOLE: 'true',
  PORTAL_STATS_ENABLED: 'true',
  PORTAL_REFERRALS_ENABLED: 'true',
};

if (args[0] !== 'worker') process.execve(binary, [binary, ...args], env);
env.PORTAL_ADMIN_CONTROL_ENABLED = 'true';

const worker = spawn(binary, args, { env, stdio: ['ignore', 'pipe', 'inherit'] });
const exited = once(worker, 'exit');
const ready = new Promise(resolve => {
  const lines = createInterface({ input: worker.stdout });
  lines.on('line', line => {
    process.stdout.write(`${line}\n`);
    if (line.includes('outbox worker started')) resolve();
  });
});
let stopping;
function stopWorker() {
  if (!stopping) {
    worker.kill('SIGTERM');
    // The production drain budget is ten seconds. Reap a child that cannot
    // handle SIGTERM so harness teardown can still close its database.
    const deadline = setTimeout(() => worker.kill('SIGKILL'), 12_000);
    stopping = exited.finally(() => clearTimeout(deadline));
  }
  return stopping;
}

// EOF acknowledges the actual worker exit while retaining serve and gateway.
const control = createServer(socket => {
  socket.on('error', () => socket.destroy());
  Promise.race([ready, exited]).then(stopWorker)
    .then(([code, signal]) => socket.end(String(code ?? signal)), () => socket.end('1'));
});
const controlFile = path.join(path.dirname(scratch), 'worker.sock');
control.listen(controlFile, () => {
  writeFileSync(path.join(scratch, 'secrets', 'worker-env.json'), JSON.stringify(env), { mode: 0o600 });
});
worker.once('error', () => process.exit(1));
worker.once('exit', (code, signal) => {
  if (code !== 0 || signal) {
    process.exitCode = code ?? 1;
    control.close();
  }
});
for (const signal of ['SIGINT', 'SIGTERM']) {
  process.once(signal, () => {
    control.close();
    void stopWorker().catch(() => { process.exitCode = 1; });
  });
}
