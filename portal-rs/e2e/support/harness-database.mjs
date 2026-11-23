import { Socket } from 'node:net';
import pg from 'pg';

export const fixtureDatabaseClaim = 'database-claim';

const fixtureDdl = Object.freeze({
  createDatabase: 'CREATE DATABASE %I',
  createOwnedDatabase: 'CREATE DATABASE %1$I OWNER %1$I',
  dropDatabase: 'DROP DATABASE %I WITH (FORCE)',
  dropDatabaseIfExists: 'DROP DATABASE IF EXISTS %I WITH (FORCE)',
  createRole: 'CREATE ROLE %I LOGIN PASSWORD %L NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS',
  dropRole: 'DROP ROLE IF EXISTS %I',
  restrictAccess: 'REVOKE CONNECT, TEMPORARY ON DATABASE %I FROM PUBLIC',
});

export async function fixtureStatement(query, operation, name, verifier = null) {
  if (!/^portal_e2e_[a-f0-9]{12}(?:_tests|_queries)?$/.test(name)) {
    throw new Error('Invalid disposable database name');
  }
  if (!Object.hasOwn(fixtureDdl, operation)) throw new Error('Unknown fixture database operation');
  // Utility statements lack bind parameters. PostgreSQL quotes these bound
  // identifiers and literals before the fixture executes the statement.
  const { rows: [{ statement }] } = await query(
    'SELECT pg_catalog.format($1::text, $2::text, $3::text) AS statement',
    [fixtureDdl[operation], name, verifier],
  );
  return statement;
}

export async function executeFixtureDdl(query, operation, name, verifier) {
  return query(await fixtureStatement(query, operation, name, verifier));
}

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

export async function removeDatabase(adminURL, dbName, onRetry = () => {}, {
  dropDatabase = true,
  retryTimeout = 15_000,
  afterDrop,
} = {}) {
  if (!/^portal_e2e_[a-f0-9]{12}$/.test(dbName)) throw new Error('Invalid disposable database name');
  const signal = AbortSignal.timeout(9_000 + retryTimeout);
  const drop = statementTimeout => withAdminConnection(adminURL, async admin => {
    await lockDatabaseName(admin, dbName);
    if (dropDatabase) {
      await executeFixtureDdl(admin.query.bind(admin), 'dropDatabaseIfExists', dbName);
      const existing = await admin.query('SELECT 1 FROM pg_database WHERE datname=$1', [dbName]);
      if (existing.rowCount !== 0) throw new Error('Disposable database still exists');
    }
    await afterDrop?.(admin);
  }, { signal, statementTimeout });
  try {
    await drop(5_000);
  } catch {
    signal.throwIfAborted();
    // A checkpoint can outlast the first statement, and a lost reply can hide
    // a successful drop. Reconcile under the same lock on a fresh connection.
    onRetry();
    await drop(retryTimeout);
  }
}
