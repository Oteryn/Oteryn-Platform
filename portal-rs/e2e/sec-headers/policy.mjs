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
