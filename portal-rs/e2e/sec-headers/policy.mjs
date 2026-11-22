import { expect } from '@playwright/test';

// crates/web/src/surface.rs defines the common policy. Module policies may
// further restrict referrers and sources. Inspect individual wire fields so
// that duplicate CSP fields and Set-Cookie attributes cannot hide a regression.
const permissions = ['camera=()', 'geolocation=()', 'microphone=()', 'payment=()', 'usb=()'];
const referrerPolicies = ['strict-origin-when-cross-origin', 'strict-origin', 'same-origin', 'no-referrer'];
export const sessionCookie = '__Host-oteryn_session';
export const deviceCookie = '__Host-oteryn_mfa_device';

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

  const policies = values('content-security-policy').flatMap(value => value.split(','))
    .map(policy => policy.split(';').map(directive => directive.trim().split(/\s+/)));
  check(policies.length, label + ' CSP policy count').toBeGreaterThan(0);
  for (const policy of policies) {
    const scriptSources = policy.filter(([name]) => /^(?:default-src|script-src(?:-elem|-attr)?)$/i.test(name));
    check(scriptSources.flat().every(source => source.toLowerCase() !== "'unsafe-inline'"),
      label + ' forbids unsafe-inline scripts').toBe(true);
  }
  check(policies.some(policy => policy.some(([name, ...sources]) =>
    name === 'frame-ancestors' && sources.length === 1 && sources[0] === "'none'"))
    || values('x-frame-options').some(value => value.toUpperCase() === 'DENY'),
  label + ' denies framing').toBe(true);

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
