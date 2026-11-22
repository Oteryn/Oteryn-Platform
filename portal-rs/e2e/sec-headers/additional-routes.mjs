// Response contracts for composed registrations absent from route-expectations.json.
// The live registration report selects which contracts become scenarios.
const playerRoutes = [
  ['admin', '/{lang}/staff-invite', 403, 'missing invitation credential'],
  ['community', '/characters/{name}/skills', 503, 'cold Skills projection'],
  ['community', '/{lang}/characters/{name}/skills', 503, 'cold Skills projection'],
  ...['achievements', 'equipment', 'outfit'].flatMap(family =>
    ['', '/{lang}'].map(prefix => [
      'community', prefix + '/account/characters/{name}/' + family, 404, 'no owned character',
    ])),
  ['community', '/{lang}/account/referrals', 200, 'account referral page'],
  ['community', '/{lang}/referrals/{code}', 403, 'missing invitation'],
  ['store', '/{lang}/shop/test-checkout/{intent}', 404, 'missing checkout intent'],
  ['web', '/{lang}/account/consents', 200, 'enabled account consent page'],
  ['web', '/{lang}/account/badges', 200, 'localized badge page'],
  ['identity', '/forgot-password', 200, 'legacy redirect reaches the password reset form'],
  ['payments', '/api/v1/native-account/coin-purchases/balance', 401, 'native bearer required'],
  ['payments', '/api/v1/native-account/coin-purchases/catalogue', 401, 'native bearer required'],
  ['payments', '/api/v1/native-account/coin-purchases/requests/{request_id}', 401, 'native bearer required'],
  ['payments', '/native-account/coin-checkout/{order_id}', 404, 'missing checkout order'],
  ['web', '/static', 404, 'static directory has no index'],
  ['web', '/static/', 404, 'static directory has no index'],
  ['web', '/static/{*asset}', 200, 'static stylesheet'],
];

const staffRoutes = [
  ['admin', '/admin/staff-invites', 200, 'staff invitation management'],
  ['content', '/admin/announcements', 200, 'announcement index'],
  ['content', '/admin/announcements/create', 200, 'new announcement form'],
  ['content', '/admin/announcements/{id}/edit', 404, 'missing announcement'],
  ['portal', '/{lang}/admin/store/codes', 200, 'store code issuance form'],
  ['admin', '/admin/support/tickets', 200, 'staff ticket queue'],
  ['admin', '/admin/support/tickets/{id}', 404, 'missing ticket'],
];

const gmRoutes = [
  ['admin', '/admin/console/enforcement', 200, 'Game viewer can open the read-only enforcement search'],
  ['admin', '/console', 303, 'the Game viewer lands on their first authorized console section'],
];

export const additionalRoutes = Object.entries({ player: playerRoutes, staff: staffRoutes, gm: gmRoutes })
  .flatMap(([actor, routes]) => routes.map(([crate, route, status, reason]) => ({ actor, crate, route, status, reason })));
