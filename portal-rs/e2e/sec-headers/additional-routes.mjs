// Concrete GET registrations absent from the checked-in inventory. The
// community family router registers two prefixes for each path template.
const playerRoutes = [
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
];

const staffRoutes = [
  ['content', '/admin/announcements', 200, 'announcement index'],
  ['content', '/admin/announcements/create', 200, 'new announcement form'],
  ['content', '/admin/announcements/{id}/edit', 404, 'missing announcement'],
  ['portal', '/{lang}/admin/store/codes', 200, 'store code issuance form'],
];

export const additionalRoutes = Object.entries({ player: playerRoutes, staff: staffRoutes })
  .flatMap(([actor, routes]) => routes.map(([crate, route, status, reason]) => ({ actor, crate, route, status, reason })));
