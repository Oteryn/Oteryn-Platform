import pg from 'pg';

const loopback = new Map([
  ['localhost', '127.0.0.1'], ['127.0.0.1', '127.0.0.1'], ['::1', '[::1]'], ['[::1]', '[::1]'],
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
  url.hostname = host;
  url.port = String(port);
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
