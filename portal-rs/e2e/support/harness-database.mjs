import { Socket } from 'node:net';
import pg from 'pg';

export const fixtureDatabaseClaim = 'database-claim';

export async function withAdminConnection(adminURL, operation, { signal, statementTimeout = 5_000 } = {}) {
  signal?.throwIfAborted();
  const socket = new Socket();
  const admin = new pg.Client({
    connectionString: adminURL.href,
    stream: socket,
    connectionTimeoutMillis: 3_000,
    statement_timeout: statementTimeout,
    query_timeout: statementTimeout + 1_500,
  });
  // A disconnect can arrive between queries. An active operation rejects
  // through its own promise, and the socket is always closed before returning.
  admin.on('error', () => {});
  const abort = () => socket.destroy();
  signal?.addEventListener('abort', abort, { once: true });
  try {
    await admin.connect();
    return await operation(admin);
  } finally {
    signal?.removeEventListener('abort', abort);
    const ended = admin.end();
    socket.destroy();
    await ended;
  }
}

// Hold the same session lock for CREATE and reconciliation. A disconnected
// creator can still be finishing its statement when the cleanup client connects.
export function lockDatabaseName(admin, dbName) {
  return admin.query('SELECT pg_advisory_lock(hashtextextended($1, 0))', [dbName]);
}

export async function removeDatabase(adminURL, dbName, onRetry = () => {}) {
  if (!/^portal_e2e_[a-f0-9]{12}$/.test(dbName)) throw new Error('Invalid disposable database name');
  const signal = AbortSignal.timeout(24_000);
  const drop = statementTimeout => withAdminConnection(adminURL, async admin => {
    await lockDatabaseName(admin, dbName);
    await admin.query('DROP DATABASE IF EXISTS "' + dbName + '" WITH (FORCE)');
    const existing = await admin.query('SELECT 1 FROM pg_database WHERE datname=$1', [dbName]);
    if (existing.rowCount !== 0) throw new Error('Disposable database still exists');
  }, { signal, statementTimeout });
  try {
    await drop(5_000);
  } catch {
    signal.throwIfAborted();
    // A checkpoint can outlast the first statement, and a lost reply can hide
    // a successful drop. Reconcile under the same lock on a fresh connection.
    onRetry();
    await drop(15_000);
  }
}
