import { routeShape } from './registrations.mjs';

// Explicit redirects on the signed-in GET sweep. Legacy fallback targets come
// separately from redirects::MAPPINGS in the server's registration report.
export function registeredRedirect(target, locale) {
  const path = target.pathname;
  const pathLocale = /^\/(pl|en)(?:\/|$)/.exec(path)?.[1] ?? locale;
  switch (routeShape(path)) {
    case '/':
      return [303, '/' + locale + '/']; // web/surface.rs
    case '/register':
    case '/{lang}/register':
      return [303, '/account']; // web/routes/register.rs
    case '/mfa/challenge':
    case '/{lang}/mfa/challenge':
      return [302, '/' + pathLocale + '/']; // identity/mfa_http.rs
    case '/account/security':
    case '/{lang}/account/security':
      return [303, '/' + pathLocale + '/password/change']; // identity/credentials_http.rs
    case '/console':
      return [303, '/console/dashboard?lang=' + locale]; // admin/console/mod.rs
    case '/{lang}/account/privacy/export':
      return [303, '/' + pathLocale + '/account/sudo?return_to=export']; // identity/export.rs
    case '/forgot-password':
      return [301, '/pl/forgot-password']; // reset_http.rs registers the legacy handler for GET
    case '/events':
      return [303, '/' + pathLocale + '/events']; // content/events/web.rs
    default:
      if (path === '/language/' + locale) return [303, '/' + locale + '/'];
  }
}
