import { Socket } from 'node:net';
import pg from 'pg';
import { disposableDatabaseUrl } from './helpers.mjs';

const loopback = new Map([
  ['localhost', '127.0.0.1'], ['127.0.0.1', '127.0.0.1'], ['::1', '::1'], ['[::1]', '::1'],
]);

export function localDatabaseURL(connectionString) {
  let url;
  try {
    url = new URL(connectionString);
  } catch {
    throw new Error('A local PostgreSQL URL is required');
  }
  const host = loopback.get(url.hostname);
  if (!['postgres:', 'postgresql:'].includes(url.protocol) || !host) throw new Error('Local PostgreSQL only');
  for (const name of ['host', 'hostaddr']) {
    for (const override of url.searchParams.getAll(name)) {
      if (!loopback.has(override)) throw new Error('Local PostgreSQL only');
      if (loopback.get(override) !== host) throw new Error('Conflicting PostgreSQL host options');
    }
    url.searchParams.delete(name);
  }
  const ports = url.searchParams.getAll('port');
  const port = Number(url.port || ports[0] || '5432');
  if (!Number.isInteger(port) || port < 1 || port > 65535
      || ports.some(value => !/^\d+$/.test(value) || Number(value) !== port)) {
    throw new Error('Invalid or conflicting PostgreSQL port options');
  }
  url.searchParams.delete('port');
  url.port = String(port);
  // SQLx uses hostaddr for TCP and the URL hostname for TLS verification.
  if (url.hostname === 'localhost') url.searchParams.set('hostaddr', host);
  // SQLx accepts dbname as an override for the disposable database pathname.
  if (url.searchParams.has('dbname')) throw new Error('Specify the PostgreSQL database in the URL path');
  let client;
  try {
    client = new pg.Client({ connectionString: url.href });
  } catch {
    throw new Error('Invalid local PostgreSQL configuration');
  }
  if (loopback.get(client.host) !== host || client.port !== port) throw new Error('Inconsistent PostgreSQL endpoint');
  return url;
}

function pinnedConnection(url) {
  const address = loopback.get(url.hostname);
  if (!address) throw new Error('Local PostgreSQL only');
  // pg ignores hostaddr. Give pg the hostname for TLS and pin its socket to
  // the same address as SQLx without querying the system name resolver.
  return {
    connectionString: url.href,
    stream: () => new class extends Socket {
      connect(port) { return super.connect(port, address); }
    }(),
  };
}

export function localDatabaseClient(url, connectionTimeoutMillis) {
  const config = pinnedConnection(url);
  const socket = config.stream();
  const client = new pg.Client({ ...config, connectionTimeoutMillis, stream: socket });
  return { client, socket };
}

export function staffDatabaseURL(connectionString) {
  const url = localDatabaseURL(connectionString);
  // The shared validator also restricts CLI URL options. Staff TLS settings
  // and transport overrides have already been handled by localDatabaseURL.
  const destination = new URL(url);
  destination.search = '';
  disposableDatabaseUrl({ PORTAL_E2E_DATABASE_URL: destination.href });
  return url;
}

export function fixtureDatabasePool(connectionString) {
  return new pg.Pool(pinnedConnection(staffDatabaseURL(connectionString)));
}
