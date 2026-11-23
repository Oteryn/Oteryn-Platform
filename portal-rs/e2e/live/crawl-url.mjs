const statefulPath = /(?:^|\/)(?:admin|internal|oauth|logout|sign-out|signout|language|verify-email|confirm-email|email-change|change-email|reset-password|password-reset|forgot-password|recovery|recover|unsubscribe|callback|challenge|revoke|restore|delete|publish|unpublish|checkout|webhooks?)(?=\/|$)/i;
const statefulQuery = /^(?:token|code|signature|action|access_token|refresh_token|id_token|state|csrf|_csrf)$/i;

function decodedPath(url) {
  try {
    const path = decodeURIComponent(url.pathname).replaceAll('\\', '/');
    return /%[\da-f]{2}/i.test(path) ? null : path;
  } catch {
    return null;
  }
}

export function credentialURL(url) {
  const path = decodedPath(url);
  return Boolean(url.username || url.password || (path !== null && statefulPath.test(path)) ||
    [...url.searchParams.keys()].some(key => statefulQuery.test(key)));
}

export function publicURL(url, origins) {
  return origins.has(url.origin) && decodedPath(url) !== null && !credentialURL(url);
}

export function diagnosticURL(value) {
  const absolute = /^(?:https?:)?\/\//i.test(String(value));
  let url;
  try {
    url = new URL(value, 'https://redacted.invalid');
  } catch {
    return '[invalid URL]';
  }
  const path = decodedPath(url);
  const sensitive = path?.match(statefulPath);
  const safePath = path === null ? '/[redacted]'
    : sensitive ? `${path.slice(0, sensitive.index + sensitive[0].length)}/[redacted]`
    : url.pathname;
  return `${absolute ? url.origin : ''}${safePath}`;
}

export function redactURLs(text) {
  return text
    .replace(/(?:https?:)?\/\/[^\s"'<>]+|(?<![\w/])\/(?!\/)[^\s"'<>]+/gi, diagnosticURL)
    .replace(/([?&][^=&\s"'<>]+)=([^&\s"'<>]+)/g, (value, key) => {
      const name = new URLSearchParams(key.slice(1) + '=').keys().next().value;
      return statefulQuery.test(name) ? `${key}=[redacted]` : value;
    });
}
