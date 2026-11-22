import { expect } from '@playwright/test';

// crates/web/src/surface.rs defines the common policy. Module policies may
// further restrict referrers and sources. Inspect individual wire fields so
// that duplicate CSP fields and Set-Cookie attributes cannot hide a regression.
const permissions = ['camera=()', 'geolocation=()', 'microphone=()', 'payment=()', 'usb=()'];
export const sessionCookie = '__Host-oteryn_session';
export const deviceCookie = '__Host-oteryn_mfa_device';

// public_http.rs and surface.rs define the listener and web policies. The
// shell adds self-only font and image sources to HTML with an explicit policy.
export const browser = "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; form-action 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'";
export const surface = browser.replace("script-src 'self'", "script-src 'self' '<nonce>'");
export const sameOrigin = "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'";
export const sameOriginForm = sameOrigin + "; form-action 'self'";
export const stylesOnly = "default-src 'none'; style-src 'self'; base-uri 'none'; frame-ancestors 'none'";
export const stylesForm = stylesOnly + "; form-action 'self'";
// atlas/web.rs issues the nonce; atlas/spawns.rs enables same-origin overlay fetches.
export const atlas = stylesForm + "; img-src 'self'; script-src '<nonce>'; connect-src 'self'";

export function shellPolicy(policy) {
  for (const name of ['font-src', 'img-src']) {
    if (!parseCsp(policy).some(([directive]) => directive === name)) policy += "; " + name + " 'self'";
  }
  return policy;
}

function parseCsp(value) {
  return value.split(';').map(part => part.trim()).filter(Boolean).map(part => {
    const [name, ...sources] = part.split(/\s+/);
    return [name.toLowerCase(), sources.map(source => /^'nonce-[a-f0-9]{64}'$/.test(source) ? "'<nonce>'" : source).sort()];
  });
}

function signature(entries) {
  return JSON.stringify([...entries].sort(([a], [b]) => a.localeCompare(b)));
}

export function matchesPortalCsp(value, expected) {
  if (value.includes("'<nonce>'")) return false;
  const directives = parseCsp(value);
  if (new Set(directives.map(([name]) => name)).size !== directives.length) return false;
  return signature(directives) === signature(parseCsp(expected));
}

// Complete non-cacheable policies emitted by the public listener and its
// modules. Asset routes with intentional public caching have their own contract.
const privateCachePolicies = [
  'no-store',
  'no-store, private',
  'max-age=0, no-store',
  'max-age=0, no-store, private',
  'no-store, no-transform',
];

export function matchesPrivateCache(fields) {
  const directives = [];
  for (const field of fields) {
    let start = 0;
    let quoted = false;
    let escaped = false;
    for (let index = 0; index < field.length; index++) {
      const char = field[index];
      if (escaped) {
        escaped = false;
      } else if (quoted && char === '\\') {
        escaped = true;
      } else if (char === '"') {
        quoted = !quoted;
      } else if (!quoted && char === ',') {
        directives.push(field.slice(start, index));
        start = index + 1;
      }
    }
    if (quoted) return false;
    directives.push(field.slice(start));
  }
  const policy = directives.map(value => value.trim().toLowerCase()).sort().join(', ');
  return privateCachePolicies.includes(policy);
}

export async function checkHeaders(response, label, {
  cache, check = expect, csp = browser, referrer = 'strict-origin-when-cross-origin',
} = {}) {
  const fields = await response.headersArray();
  const values = name => fields.filter(field => field.name.toLowerCase() === name).map(field => field.value);
  check(values('x-content-type-options'), label + ' nosniff').toEqual(['nosniff']);
  check(values('strict-transport-security'), label + ' HTTPS HSTS')
    .toEqual(['max-age=63072000; includeSubDomains']);
  check(values('referrer-policy'), label + ' referrer policy').toEqual([referrer]);
  check(values('permissions-policy').flatMap(value => value.split(',').map(part => part.trim())).sort(),
    label + ' permissions policy').toEqual(permissions);

  const policies = values('content-security-policy');
  check(policies.length, label + ' one enforced CSP').toBe(1);
  for (const policy of policies) {
    check(matchesPortalCsp(policy, csp), label + ' exact CSP directives and sources (' + csp + ')').toBe(true);
  }

  if (cache) {
    check(values('cache-control'), label + ' configured asset cache').toEqual([cache]);
  } else {
    check(matchesPrivateCache(values('cache-control')), label + ' exact non-cacheable directives').toBe(true);
  }

  return values('set-cookie').map(field => checkCookieField(field, label, check));
}

export function checkCookieField(field, label, check = expect) {
  const [pair, ...parts] = field.split(';');
  const separator = pair.indexOf('=');
  const attributes = new Map();
  for (const part of parts) {
    const attribute = part.trim();
    const equals = attribute.indexOf('=');
    const name = equals === -1 ? attribute : attribute.slice(0, equals);
    const value = equals === -1 ? '' : attribute.slice(equals + 1);
    const key = name.trim().toLowerCase();
    // Duplicate path/domain/flag directives can be interpreted differently by clients.
    expect(attributes.has(key), 'unique Set-Cookie attribute').toBe(false);
    attributes.set(key, value);
  }
  const name = pair.slice(0, separator);
  const cookie = { name, isEmpty: pair.length === separator + 1, attributes };
  if ([sessionCookie, deviceCookie, 'portal_language'].includes(name)) assertCookie(cookie, label, check);
  return cookie;
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
