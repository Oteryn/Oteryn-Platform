"""Check the generated registrations against the matrix fixture declarations."""
import json
import re
import sys
from pathlib import Path


root = Path(__file__).resolve().parent
inventory = json.loads((root / 'routes.json').read_text())
routes = {(row['listener'], row['method'], row['path']): row for row in inventory['routes']}
covered = {}


def add(listener, method, path, group):
    key = listener, method, path
    if key not in routes:
        raise ValueError(f'Case or limitation has no registration: {key}')
    covered.setdefault(key, group)


def array(name, group):
    source = (root / name).read_text()
    for method, path in re.findall(r"\['(GET|POST|DELETE|PUT|PATCH|HEAD)', '(/[^']*)'\]", source):
        add('public', method, path, group)


for f in ['public-reads', 'protected-reads']:
    for r in json.loads((root/(f+'.json')).read_text()):
        method, path = r.get('method', 'GET'), r['path']
        add('public', method, path, f)
        head = ('public', 'HEAD', path)
        if head in routes and method == 'GET':
            add(*head, f)
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
for m in ['CONNECT','DELETE','HEAD','OPTIONS','PATCH','PUT','TRACE']:
    add('public',m,'/native-account/coin-checkout/{order_id}','checkout-unavailable')
for prefix in ['', '/pl','/en']:
    for p in ['/login','/mfa/login','/account/security/email','/register/abuse/challenge','/verify-email/resend']:
        add('public','POST',prefix+p,'account-workflows')
    for m in ['POST','PUT']: add('public',m,prefix+'/password/change','passwords')
for m in ['GET','POST']: add('public',m,'/native-account/coin-checkout/{order_id}','native-checkout')
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
    file, line = r['source'].rsplit(':', 1)
    source = root.parents[1] / file
    if not r['reason'].strip() or not (0 < int(line) <= len(source.read_text().splitlines())):
        raise ValueError(f"Invalid local limitation: {r['method']} {r['path']}")
    key = (r['listener'], r['method'], r['path'])
    if key in covered:
        raise ValueError(f'Executable route also has a local limitation: {key}')
    add(*key, 'limitation')

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
    if r['status'] != 401 and not r.get('localLimit'):
        raise ValueError(f"Missing authority limitation for {r['method']} {r['path']}")
missing = sorted(routes.keys() - covered.keys())
for key in missing:
    source = routes[key]['registrations'][0]['source']
    print(*key, source, file=sys.stderr)
if missing:
    sys.exit(f"{len(missing)} effective routes need a case or a source-backed local limitation")
print(f"{len(covered)} route/method entries accounted for")
