import pg from 'pg';

const loopback = new Set(['localhost', '127.0.0.1', '::1', '[::1]']);

export function localDatabaseURL(connectionString) {
  let url;
  try {
    url = new URL(connectionString);
  } catch {
    throw new Error('A local PostgreSQL URL is required');
  }
  // Check every host value because pg and the portal can parse URL options
  // differently. Check pg's resolved host before either launcher connects.
  if (!['postgres:', 'postgresql:'].includes(url.protocol)
      || !loopback.has(url.hostname)
      || [...url.searchParams].some(([name, value]) =>
        ['host', 'hostaddr'].includes(name) && !loopback.has(value))) {
    throw new Error('Local PostgreSQL only');
  }
  // SQLx accepts dbname as an override for the disposable database pathname.
  if (url.searchParams.has('dbname')) throw new Error('Specify the PostgreSQL database in the URL path');
  let host;
  try {
    host = new pg.Client({ connectionString }).host;
  } catch {
    throw new Error('Invalid local PostgreSQL configuration');
  }
  if (!loopback.has(host)) throw new Error('Local PostgreSQL only');
  return url;
}
