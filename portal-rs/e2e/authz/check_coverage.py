"""Check the generated registrations against the matrix fixture declarations."""
import json
import re
import sys
from pathlib import Path


root = Path(__file__).resolve().parent
inventory = json.loads((root / 'routes.json').read_text())
routes = {(row['listener'], row['method'], row['path']): row for row in inventory['routes']}
covered = {}
covered_profiles = set()


def add(listener, method, path, group, profile=None):
    key = listener, method, path
    if key not in routes:
        raise ValueError(f'Case or limitation has no registration: {key}')
    profiles = {item.get('profile') for item in routes[key]['registrations']}
    if profile not in profiles:
        raise ValueError(f'Case or limitation has no registration in profile {profile}: {key}')
    if profile:
        covered_profiles.add((*key, profile))
    covered.setdefault(key, group)


def array(name, group):
    source = (root / name).read_text()
    for method, path in re.findall(r"\['(GET|POST|DELETE|PUT|PATCH|HEAD)', '(/[^']*)'\]", source):
        profile = None
        if '/bazaar/items' in path:
            profile = 'bazaar-unavailable' if group == 'checkout-unavailable.spec' else 'bazaar-runtime'
        add('public', method, path, group, profile)
        if profile and method == 'GET':
            add('public', 'HEAD', path, group, profile)


def valid_source(reference):
    file, line = reference.rsplit(':', 1)
    return 0 < int(line) <= len((root.parents[1] / file).read_text().splitlines())


for f in ['public-reads', 'protected-reads']:
    for r in json.loads((root/(f+'.json')).read_text()):
        method, path = r.get('method', 'GET'), r['path']
        profile = 'bazaar-runtime' if path == '/{locale}/bazaar/items' else None
        add('public', method, path, f, profile)
        head = ('public', 'HEAD', path)
        if head in routes and method == 'GET':
            add(*head, f, profile)
src=(root/'matrix.spec.mjs').read_text()
m = re.search(r"const browserRoutes = \[.*?\.flatMap", src, re.S)[0]
for p in re.findall(r"'(/[^']*)'", m):
    for method in ('GET', 'HEAD'):
        add('public', method, p, 'matrix')
for p in ['/admin/roles','/__test/payments/webhook']: add('public','POST',p,'matrix')
for listener,m,p in routes:
    r=routes[listener,m,p]
    protected_private = listener in {'internal', 'admission'} and any(
        item.get('refusal') for item in r['registrations'])
    isolated_private = listener in {'bazaar', 'closure', 'league'}
    health = listener in {'gateway', 'admission'} and m in {'GET', 'HEAD'}
    if protected_private or isolated_private or health:
        add(listener,m,p,'matrix-protocol')
src=(root/'protected.spec.mjs').read_text()
for m,p in re.findall(r"method: '(GET|POST|DELETE|PUT|PATCH|HEAD)', path: '(/[^']*)'",src):
    add('public',m,p,'identity')
for m in ['POST','DELETE']: add('public',m,'/api/v1/stats/watches','identity')
for loc in ['', '/pl','/en']:
    add('public','POST',loc+'/account/sudo','identity')
    if loc: add('public','GET',loc+'/account/privacy/export','identity')
for name in ['editorial','support','calendar','resource-reads','staff-actions','referrals','shop','party','checkout-unavailable.spec','oauth','commerce']:
    array(name+'.mjs',name)
for loc in ['', '/pl','/en']:
    for m,s in [('DELETE',''),('POST','/others'),('POST','/{id}'),('DELETE','/{id}'),('POST','/all')]:
        add('public',m,loc+'/account/sessions'+s,'sessions')
    add('public','POST',loc+'/logout','sessions')
    for m,s in [('POST',''),('POST','/confirm'),('POST','/revoke'),('DELETE','')]:
        add('public',m,loc+'/account/recovery-key'+s,'recovery')
src=(root/'protocol.mjs').read_text()
for m,p in re.findall(r"method: '(GET|POST|DELETE|PUT|PATCH|HEAD)', path: '(/[^']*)'",src):
    add('public',m,p,'protocol')
for l,m,p in re.findall(r"\['(public|native)', '(GET|POST)', '(/[^']*)'\]",src):
    add(l,m,p,'protocol')
for m,s in [('POST',''),('GET','/{id}/edit'),('GET','/{id}/preview/{locale}'),('POST','/{id}'),('GET','/{id}/revisions'),('POST','/{id}/revisions/{source}/restore'),('POST','/{id}/{action}')]:
    add('public',m,'/admin/wiki/articles'+s,'wiki')
for p in ['/wiki/articles/{slug}','/{locale}/wiki/articles/{slug}']:
    add('public','GET',p,'wiki')
limits=json.loads((root/'local-limitations.json').read_text())
for loc in ['', '/pl','/en']:
 for family in ['achievements','equipment','outfit']:
    add('public','GET',loc+'/account/characters/{name}/'+family,'character-families')
for area in ['dashboard','worlds','nodes','database']:
    add('public','POST','/console/'+area,'console')
for prefix in ['', '/pl','/en']:
    add('public','POST',prefix+'/account/badges','badges')
for f in ['collection','journal','feature-gates']:
 for method,path in re.findall(r"method: '(GET|POST)', path: '(/[^']*)'",(root/(f+'.mjs')).read_text()):
    add('public',method,path,f)
for p in re.findall(r"\['(/updates/[^']*)',",(root/'downloads.mjs').read_text()):
    add('public','GET',p,'downloads')
for p in ['/v1/login','/v1/recover','/v1/recover/revoke']:
    add('gateway','POST',p,'gateway')
checkout = '/native-account/coin-checkout/{order_id}'
for method in ('CONNECT', 'DELETE', 'GET', 'HEAD', 'OPTIONS', 'PATCH', 'POST', 'PUT', 'TRACE'):
    registrations = routes['public', method, checkout]['registrations']
    expected = {'unavailable': 'anonymous'}
    if method in {'GET', 'HEAD', 'POST'}:
        expected['configured'] = 'logged-in'
    if {row.get('profile'): row['guard']['kind'] for row in registrations} != expected:
        raise ValueError(f'Unexpected checkout profiles for {method} {checkout}')
    add('public', method, checkout, 'checkout-unavailable', 'unavailable')
    if 'configured' in expected:
        add('public', method, checkout, 'native-checkout', 'configured')
for prefix in ['', '/pl','/en']:
    for p in ['/login','/mfa/login','/account/security/email','/register/abuse/challenge','/verify-email/resend']:
        add('public','POST',prefix+p,'account-workflows')
    for m in ['POST','PUT']: add('public',m,prefix+'/password/change','passwords')
for p in ['/admin/console/players','/admin/control/step-up']:
    add('public','POST',p,'console')
for s in ['', '/cancel']:
    add('public','POST','/{lang}/account/privacy/delete'+s,'termination')
add('public','GET','/oauth/authorize','oauth')
for method in ('GET', 'HEAD', 'POST'):
    add('public', method, '/{lang}/admin/store/codes', 'store-codes')
add('public', 'POST', '/{lang}/shop/codes/redeem', 'store-codes')
for method in ('GET', 'HEAD'):
    add('public', method, '/wiki/{slug}', 'wiki')
for path in ('/{locale}/account/bazaar/sell', '/{locale}/account/bazaar/{listing}/bids'):
    add('public', 'POST', path, 'bazaar')
for r in limits:
    if not r['reason'].strip() or not valid_source(r['source']):
        raise ValueError(f"Invalid local limitation: {r['method']} {r['path']}")
    key = (r['listener'], r['method'], r['path'])
    profile = r.get('profile')
    if profile and not any(item.get('profile') == profile and item['source'] == r['source']
                           for item in routes[key]['registrations']):
        raise ValueError(f'Profile limitation must name its specific registration: {key}')
    if (*key, profile) in covered_profiles or (not profile and key in covered):
        raise ValueError(f'Executable route also has a local limitation: {key}')
    add(*key, 'limitation', profile)
    head = routes.get((key[0], 'HEAD', key[2]))
    if profile and key[1] == 'GET' and head and any(
        item.get('profile') == profile and item.get('implicitFrom') == 'GET' for item in head['registrations']
    ):
        add(key[0], 'HEAD', key[2], 'limitation', profile)

# Feature fixtures that also probe HEAD using the GET outcome and the same target.
with_head = {
    'calendar', 'character-families', 'checkout-unavailable.spec', 'collection', 'downloads',
    'editorial', 'feature-gates', 'identity', 'journal', 'native-checkout', 'oauth', 'protocol',
    'referrals', 'resource-reads', 'shop', 'staff-actions', 'support', 'wiki',
}
for (listener, method, path), group in list(covered.items()):
    if method != 'GET' or group not in with_head | {'limitation'}:
        continue
    row = routes.get((listener, 'HEAD', path))
    if row and any(item.get('profile') for item in row['registrations']):
        continue  # Profile-specific HEAD cases are declared alongside their GETs.
    if row and (group != 'limitation' or all(item.get('implicitFrom') == 'GET' for item in row['registrations'])):
        add(listener, 'HEAD', path, group)

private_http = json.loads((root / 'private-http.json').read_text())
registered_private = {(method, path) for listener, method, path in routes if listener == 'internal'}
probed_private = {(r['method'], r['path']) for r in private_http}
if registered_private != probed_private or len(probed_private) != len(private_http):
    raise ValueError('CA-valid private HTTP probes must match the internal inventory exactly')
for r in private_http:
    if r['status'] == 401 and not (r.get('authorizedPeer') and r.get('authorizedInvalidBodyStatus') == 400):
        raise ValueError(f"Missing admitted peer fixture for {r['method']} {r['path']}")
    if r['status'] != 401 and not (r.get('localLimit') and r.get('limitSource') and valid_source(r['limitSource'])):
        raise ValueError(f"Missing authority limitation for {r['method']} {r['path']}")
missing = sorted(routes.keys() - covered.keys())
profile_registrations = [
    (*key, item['profile'])
    for key, row in routes.items() for item in row['registrations'] if item.get('profile')
]
expected_profiles = set(profile_registrations)
if len(expected_profiles) != len(profile_registrations):
    raise ValueError('Each listener, path, method and startup profile must identify one registration')
uncovered_profiles = sorted(expected_profiles - covered_profiles)
if uncovered_profiles:
    sys.exit(f'Profile registrations need a case or source-backed limitation: {uncovered_profiles}')
for key in missing:
    source = routes[key]['registrations'][0]['source']
    print(*key, source, file=sys.stderr)
if missing:
    sys.exit(f"{len(missing)} effective routes need a case or a source-backed local limitation")
print(f"{len(covered)} route/method entries and {len(covered_profiles)} startup-profile registrations accounted for")
