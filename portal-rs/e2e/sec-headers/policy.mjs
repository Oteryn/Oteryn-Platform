import { expect } from '@playwright/test';

// crates/web/src/surface.rs defines the common policy. Module policies may
// further restrict referrers and sources. Inspect individual wire fields so
// that duplicate CSP fields and Set-Cookie attributes cannot hide a regression.
const permissions = ['camera=()', 'geolocation=()', 'microphone=()', 'payment=()', 'usb=()'];
const referrerPolicies = ['strict-origin-when-cross-origin', 'strict-origin', 'same-origin', 'no-referrer'];
export const sessionCookie = '__Host-oteryn_session';
export const deviceCookie = '__Host-oteryn_mfa_device';

// Exact directive/source sets from public_http.rs, surface.rs, and the
// module policies. chrome.rs adds self-only font/img directives to HTML.
const browser = "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; form-action 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'";
const sameOrigin = "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'";
const stylesOnly = "default-src 'none'; style-src 'self'; base-uri 'none'; frame-ancestors 'none'";
const icon = "default-src 'none'; sandbox; frame-ancestors 'none'";
const modulePolicies = [
  sameOrigin,
  sameOrigin + "; form-action 'self'",
  sameOrigin + "; img-src 'self' data:",
  sameOrigin.replace("base-uri 'none'", "base-uri 'self'") + "; form-action 'self'", // staff media
  stylesOnly,
  stylesOnly + "; form-action 'self'",
  stylesOnly + "; form-action 'self'; img-src 'self'", // worlds
  stylesOnly + "; form-action 'self'; img-src 'self'; script-src '<nonce>'", // atlas
];

function parseCsp(value) {
  return value.split(';').map(part => part.trim()).filter(Boolean).map(part => {
    const [name, ...sources] = part.split(/\s+/);
    return [name.toLowerCase(), sources.map(source => /^'nonce-[a-f0-9]{64}'$/.test(source) ? "'<nonce>'" : source).sort()];
  });
}

function signature(entries) {
  return JSON.stringify([...entries].sort(([a], [b]) => a.localeCompare(b)));
}

function shellPolicy(value) {
  const result = parseCsp(value);
  for (const name of ['font-src', 'img-src']) {
    if (!result.some(([directive]) => directive === name)) result.push([name, ["'self'"]]);
  }
  return result;
}

const expectedPolicies = new Set([
  parseCsp(browser),
  parseCsp(browser.replace("script-src 'self'", "script-src 'self' '<nonce>'")),
  parseCsp(icon),
  ...modulePolicies.flatMap(policy => [parseCsp(policy), shellPolicy(policy)]),
].map(signature));

export function matchesPortalCsp(value) {
  const directives = parseCsp(value);
  return new Set(directives.map(([name]) => name)).size === directives.length
    && expectedPolicies.has(signature(directives));
}

export async function checkHeaders(response, label, { cache, check = expect } = {}) {
  const fields = await response.headersArray();
  const values = name => fields.filter(field => field.name.toLowerCase() === name).map(field => field.value);
  check(values('x-content-type-options'), label + ' nosniff').toEqual(['nosniff']);
  check(values('strict-transport-security'), label + ' HTTPS HSTS')
    .toEqual(['max-age=63072000; includeSubDomains']);
  const referrer = values('referrer-policy');
  check(referrer.length === 1 && referrerPolicies.includes(referrer[0]), label + ' referrer policy').toBe(true);
  check(values('permissions-policy').flatMap(value => value.split(',').map(part => part.trim())).sort(),
    label + ' permissions policy').toEqual(permissions);

  const policies = values('content-security-policy');
  check(policies.length, label + ' one enforced CSP').toBe(1);
  for (const policy of policies) {
    check(matchesPortalCsp(policy), label + ' configured CSP directives and sources').toBe(true);
  }

  if (cache) {
    check(values('cache-control'), label + ' configured asset cache').toEqual([cache]);
  } else {
    const directives = values('cache-control').join(',').split(',').map(value => value.trim().toLowerCase());
    check(directives, label + ' no-store').toContain('no-store');
    check(directives.some(value => /^(?:public|s-maxage\s*=|immutable)/.test(value)),
      label + ' no shared cache opt-in').toBe(false);
  }

  const cookies = values('set-cookie').map(parseCookie);
  for (const cookie of cookies.filter(cookie => [sessionCookie, deviceCookie, 'portal_language'].includes(cookie.name))) {
    assertCookie(cookie, label, check);
  }
  return cookies;
}

function parseCookie(field) {
  const [pair, ...parts] = field.split(';');
  const separator = pair.indexOf('=');
  const attributes = new Map();
  for (const part of parts) {
    const [name, value = ''] = part.trim().split('=', 2);
    const key = name.toLowerCase();
    // Duplicate path/domain/flag directives can be interpreted differently by clients.
    expect(attributes.has(key), 'unique Set-Cookie attribute').toBe(false);
    attributes.set(key, value);
  }
  return { name: pair.slice(0, separator), isEmpty: pair.length === separator + 1, attributes };
}

function assertCookie({ name, attributes }, label, check) {
  check(attributes.has('secure'), label + ' ' + name + ' Secure').toBe(true);
  check(attributes.has('httponly'), label + ' ' + name + ' HttpOnly').toBe(true);
  check(attributes.get('samesite'), label + ' ' + name + ' SameSite').toBe('Lax');
  check(attributes.get('path'), label + ' ' + name + ' Path').toBe('/');
  check(attributes.has('domain'), label + ' ' + name + ' host-only').toBe(false);
}

export function checkIssued(cookies, name, maxAge) {
  const matching = cookies.filter(cookie => cookie.name === name);
  expect(matching.length, name + ' Set-Cookie count').toBe(1);
  expect(matching[0].isEmpty, name + ' empty value').toBe(maxAge === '0');
  expect(matching[0].attributes.get('max-age'), name + ' Max-Age').toBe(maxAge);
  if (maxAge === undefined) {
    expect(matching[0].attributes.has('expires'), name + ' has browser-session lifetime').toBe(false);
  }
}
