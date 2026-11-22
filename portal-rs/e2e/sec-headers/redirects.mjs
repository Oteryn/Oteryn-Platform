import { routeShape } from './registrations.mjs';

// Explicit redirects on the signed-in GET sweep. Legacy fallback targets come
// separately from redirects::MAPPINGS in the server's registration report.
export function registeredRedirect(target, locale) {
  const path = target.pathname;
  switch (routeShape(path)) {
    case '/':
      return [303, '/' + locale + '/']; // web/surface.rs
    case '/register':
    case '/{lang}/register':
      return [303, '/account']; // web/routes/register.rs
    case '/mfa/challenge':
    case '/{lang}/mfa/challenge':
      return [302, '/' + locale + '/']; // identity/mfa_http.rs
    case '/account/security':
    case '/{lang}/account/security':
      return [303, '/' + locale + '/password/change']; // identity/credentials_http.rs
    case '/forgot-password':
      return [301, '/pl/forgot-password']; // reset_http.rs registers the legacy handler for GET
    case '/events':
      return [303, '/pl/events']; // content/events/web.rs
    default:
      if (path === '/language/' + locale) return [303, '/' + locale + '/'];
      if (path.startsWith('/events/')) return [303, '/pl' + path];
  }
}
