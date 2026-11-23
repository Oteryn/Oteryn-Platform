export const stagingOrigin = 'https://staging.oteryn.com';
export const stagingHost = new URL(stagingOrigin).host;
export const mailList = '/__mailpit/api/v1/messages';

const proxy = process.env.HTTPS_PROXY || process.env.https_proxy;
export const stagingProxy = proxy ? { server: proxy } : undefined;

export function accessHeaders() {
  const id = process.env.STAGING_CF_ACCESS_CLIENT_ID;
  const secret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
  if (!id || !secret) {
    throw new Error('Set STAGING_CF_ACCESS_CLIENT_ID and STAGING_CF_ACCESS_CLIENT_SECRET.');
  }
  return { 'CF-Access-Client-Id': id, 'CF-Access-Client-Secret': secret };
}

export function stagingURL(target) {
  const url = new URL(target, stagingOrigin);
  if (url.origin !== stagingOrigin || url.username || url.password) {
    throw new Error('Expected a URL on the staging origin.');
  }
  return url;
}

export function safePath(target) {
  return stagingURL(target).pathname.replace(/(\/reset-password\/)[^/]+$/, '$1[token]');
}

export async function requestTo(request, method, target, options = {}) {
  const url = stagingURL(target);
  if (url.pathname.startsWith('/__mailpit/') && method !== 'GET') {
    throw new Error('Mailpit permits GET requests only.');
  }
  try {
    return await request.fetch(url.href, {
      ...options,
      headers: { ...options.headers, ...accessHeaders() },
      method,
      maxRedirects: 0,
      timeout: options.timeout ?? 12_000,
    });
  } catch {
    // Playwright API errors can include the request's header values.
    throw new Error(`${method} ${safePath(target)} failed before receiving an HTTP response.`);
  }
}
