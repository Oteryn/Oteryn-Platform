import { atlas, browser, sameOrigin, sameOriginForm, shellPolicy, stylesForm, stylesOnly, surface } from './policy.mjs';
import { routeShape } from './registrations.mjs';

const localRoute = route => route.replace(/^\/\{lang\}(?=\/|$)/, '');
const matches = (pattern, path) => pattern.test(path);

// Separate from the status inventory. These select the exact policy set by
// each owner before the shared shell adds its local font/image sources.
export function routePolicy(contract, status, matched) {
  const route = routeShape(matched ?? contract.route);
  const local = localRoute(route);
  const redirect = status >= 300 && status < 400;

  // web/surface.rs owns the ordinary web router, the legacy fallback, and
  // feature modules explicitly composed through ratelimit::finish_routes.
  if (contract.location || contract.legacyFallback) return { csp: surface };
  if (local === '/favicon.ico') return { csp: "default-src 'none'; sandbox; frame-ancestors 'none'" };
  if (matches(/^\/mfa(?:\/|$)/, local)) return { csp: redirect ? sameOrigin : shellPolicy(sameOrigin) };
  if (local === '/oauth/authorize') return { csp: shellPolicy(stylesForm), referrer: 'no-referrer' };
  if (matches(/^\/api\/v1\/native-account\/coin-purchases\/|^\/native-account\/coin-checkout\//, local)) {
    return { referrer: 'no-referrer' }; // payments/coin_http.rs and public_http.rs
  }
  if (matches(/^\/(?:forgot-password|reset-password(?:\/\{\})?)$/, local)) {
    return { csp: surface, ...(status === 200 && { referrer: 'no-referrer' }) }; // identity/reset_http.rs
  }
  if (matches(/^\/account\/security\/email$|^\/email-change\/(?:confirm|revert)\//, local)) {
    return { csp: surface, ...(status === 200 && { referrer: 'same-origin' }) }; // identity/email_change_http.rs
  }
  if (local === '/account/security/wizard') return { csp: shellPolicy(stylesOnly), referrer: 'no-referrer' };
  if (local === '/account/profile') return { csp: browser };
  if (local === '/staff-invite') return { csp: shellPolicy(sameOriginForm), referrer: 'strict-origin' };
  if (local === '/account/passkeys' || local === '/account/passkeys/manage') return { csp: shellPolicy(sameOrigin), referrer: 'strict-origin' };
  if (local === '/account/privacy') return { csp: shellPolicy(sameOrigin) }; // identity/privacy_web.rs
  if (matches(/^\/account\/referrals$|^\/referrals\//, local)) {
    return { csp: shellPolicy(stylesForm), referrer: 'same-origin' }; // community/referrals/web.rs
  }
  if (matches(/^\/account\/characters\/\{\}\/(?:achievements|equipment|outfit)$|^\/characters\/\{\}\/skills$/, local)) {
    return { csp: shellPolicy(sameOrigin), referrer: 'no-referrer' }; // community/families/web.rs
  }
  if (matches(/^\/characters(?:\/\{\})?$/, local) && route.startsWith('/{lang}/')) {
    return { csp: shellPolicy(stylesForm), referrer: 'no-referrer' }; // community/profiles.rs
  }
  if (matches(/^(?:\/characters\/\{\})?\/deaths$/, local)) return { csp: shellPolicy(stylesForm) };
  if (local === '/worlds' || local === '/api/v1/worlds') {
    const worlds = stylesForm + "; img-src 'self'";
    return { csp: local === '/worlds' ? shellPolicy(worlds) : worlds }; // community/worlds.rs
  }
  if (matches(/^\/(?:announcements\/ticker|atlas\/delivery|account\/dashboard|wiki\/places)$|^\/(?:rankings|guilds|online|houses|today|stats)\b/, local)) {
    return { csp: shellPolicy(sameOrigin) }; // community, stats, content and atlas page policies
  }
  if (local === '/atlas' && status === 200) return { csp: shellPolicy(atlas) }; // atlas/web.rs
  if (matches(/^\/(?:panel\/payments|panel\/premium)(?:\/|$)/, local)) {
    return { csp: shellPolicy(sameOrigin), referrer: 'no-referrer' }; // payments browser middleware
  }
  if (matches(/^\/(?:puzzles|support\/reports)(?:\/|$)/, local)) {
    return { csp: shellPolicy(sameOriginForm), referrer: 'same-origin' };
  }
  if (matches(/^\/collection(?:-pvp|\/|$)/, local)) {
    return { csp: shellPolicy(sameOriginForm), referrer: 'no-referrer' }; // minigames collection HTTP
  }
  if (local === '/static/playerdata/hunts.css') return { csp: stylesForm, referrer: 'no-referrer' };
  if (matches(/^\/account\/hunts(?:\/|$)/, local)) return { csp: shellPolicy(stylesForm), referrer: 'no-referrer' };
  if (local === '/admin/console/enforcement') {
    return { csp: shellPolicy(sameOriginForm), referrer: 'no-referrer' };
  }
  if (status === 200 && matches(/^\/admin\/console\/(?:access|approvals|audit|players)$/, local)) {
    return { csp: shellPolicy(sameOriginForm), referrer: 'no-referrer' };
  }
  if (matches(/^\/admin\/media(?:\/|$)/, local)) {
    return { csp: shellPolicy(sameOriginForm), referrer: 'same-origin' };
  }
  if (matches(/^\/account\/journal\/reports|^\/(?:admin|console)\b|^\/\{lang\}\/admin\b/, local)) {
    const nonDocument = redirect || matches(/^\/admin\/analytics(?:\/[^/]+)?\/data$|^\/admin\/store\/codes$|\/notes\/export$/, local);
    return { csp: nonDocument ? sameOriginForm : shellPolicy(sameOriginForm) };
  }
  if (matches(/^\/(?:events|news|pages|help|rules|server-information|legal|wiki|support|party|shop)(?:\/|$)/, local)) {
    return { csp: redirect ? sameOriginForm : shellPolicy(sameOriginForm) }; // content, Wiki, support, party and store
  }
  if (matches(/^\/(?:account|register|verify-email|recovery-key|password\/change|unsubscribe|search|static|healthz|readyz|language)(?:\/|$)/, local)
      && !matches(/^\/account\/(?:bazaar|hunts|privacy\/export)(?:\/|$)|^\/static\/playerdata\//, local)) {
    return { csp: surface };
  }
  if (route === '/{lang}' || route === '/{lang}/' || route === '/') return { csp: surface };
  return { csp: browser }; // public_http.rs and chrome.rs share the strict listener fallback
}
