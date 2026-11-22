import { readFileSync } from 'node:fs';

// Axum 0.8's Debug representation is also used by the binary's flag-boundary
// audit. Reject schema drift or a missing route ID instead of dropping routes.
function parseRouter(router, routerIndex) {
  const sections = /^Router \{ path_router: PathRouter \{ routes: \{(.*?)\}, node: Node \{ paths: \{(.*?)\} \} \}, fallback_router: /s.exec(router);
  if (!sections) throw new Error('Invalid composed public router report');
  const paths = new Map([...sections[2].matchAll(/RouteId\((\d+)\): "([^"]+)"/g)].map(([, id, path]) => [id, path]));
  const endpoints = [...sections[1].matchAll(/RouteId\((\d+)\): (.*?)(?=, RouteId\(|$)/gs)];
  if (paths.size === 0 || paths.size !== endpoints.length) throw new Error('Public router registration count mismatch');
  const registrations = endpoints.map(([, id, endpoint]) => {
    const path = paths.get(id);
    if (!paths.delete(id)) throw new Error('Unknown or duplicate Axum route ID');
    if (endpoint.startsWith('Route(')) return { path, routerIndex, get: true };
    const method = /^MethodRouter\(MethodRouter \{ get: (.*?), head: /s.exec(endpoint);
    if (!method) throw new Error('Unrecognized Axum method registration for ' + path);
    return { path, routerIndex, get: method[1] !== 'None' || endpoint.includes('allow_header: Skip') };
  });
  return registrations;
}

export function publicRegistrations(file = process.env.PORTAL_E2E_ROUTE_REPORT) {
  if (!file) throw new Error('Run bash sec-headers/launch.sh, then pass its env.json to run.mjs');
  const { routers, legacy, mfaLoginRewrite } = JSON.parse(readFileSync(file, 'utf8'));
  if (!Array.isArray(routers) || !routers.length || !Array.isArray(legacy) || !legacy.length) {
    throw new Error('Incomplete composed public router report');
  }
  const seen = new Set();
  const registrations = routers.flatMap(parseRouter).filter(({ path }) => {
    if (seen.has(path)) return false;
    seen.add(path);
    return true;
  });
  if (typeof mfaLoginRewrite !== 'boolean' || routers.length !== (mfaLoginRewrite ? 2 : 1)) {
    throw new Error('Missing MFA composition in the public router report');
  }
  return { registrations: registrations.sort((a, b) => a.path.localeCompare(b.path)), legacy, mfaLoginRewrite };
}

export function routeShape(path) {
  return path.replace(/^\/(?:en|pl)(?=\/|$)/, '/{lang}')
    .replace(/\{([^}]+)\}/g, (_, name) => ['lang', 'locale', 'language'].includes(name) ? '{lang}' : name.startsWith('*') ? '{*}' : '{}');
}

export function projectPath(path, locale) {
  return !/^\/(?:en|pl)(?=\/|$)/.test(path) || path.split('/')[1] === locale;
}

// Match the literal, parameter and catch-all precedence in matchit. Legacy
// aliases can be shadowed by a concrete endpoint, including a POST-only route.
export function registrationFor(registrations, path) {
  const segments = path.split('/');
  const matches = [];
  for (const registration of registrations) {
    const pattern = registration.path.split('/');
    let priority = '';
    for (const [index, part] of pattern.entries()) {
      if (part.startsWith('{*') && segments.slice(index).join('/')) {
        matches.push({ registration, priority: priority + '1' });
        break;
      }
      if (part === segments[index]) priority += '3';
      else if (part.startsWith('{') && segments[index]) priority += '2';
      else break;
      if (index === pattern.length - 1 && pattern.length === segments.length) {
        matches.push({ registration, priority });
      }
    }
  }
  matches.sort((a, b) => a.registration.routerIndex - b.registration.routerIndex || b.priority.localeCompare(a.priority));
  return matches[0]?.registration;
}

export const matchedRouteHeader = 'x-portal-e2e-matched-route';
export const ordinaryLogin = path => /^\/(?:pl\/|en\/)?login$/.test(path);

export function publicRoute(report, path) {
  const beforeRewrite = registrationFor(report.registrations, path);
  if (!report.mfaLoginRewrite || beforeRewrite?.routerIndex === 0 || !ordinaryLogin(path)) {
    return beforeRewrite;
  }
  return registrationFor(report.registrations, path.replace(/\/login$/, '/mfa/login'));
}
