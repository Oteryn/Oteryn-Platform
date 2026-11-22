// The harness replaces the database path after CREATE DATABASE. Keep the URL
// unambiguous for both node-postgres and SQLx, which interpret query options
// differently and can let them take precedence over the URL authority/path.
export function localPostgresUrl(raw) {
  try {
    const url = new URL(raw);
    if (!['postgres:', 'postgresql:'].includes(url.protocol) ||
        !['localhost', '127.0.0.1', '[::1]'].includes(url.hostname) ||
        url.search || url.hash || !/^\/[^/\0]+$/.test(decodeURIComponent(url.pathname))) {
      throw new Error();
    }
    url.port ||= '5432';
    url.search = '';
    url.hash = '';
    return url;
  } catch {
    throw new Error('Expected a local PostgreSQL URL with an explicit database path and no query parameters or fragment');
  }
}
