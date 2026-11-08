"""Source inventory; locale/prefix aliases share one matrix row."""
from pathlib import Path
import json
import re

ROOT = Path(__file__).resolve().parents[1]

def canonical(path):
    path = path.replace('{{', '{').replace('}}', '}')
    path = re.sub(r'^/(pl|en)(?=/|$)', '/{lang}', path)
    return path.replace('{locale}', '{lang}').replace('{language}', '{lang}').replace('{prefix}', '/{lang}')

rows = {}
for file in sorted((ROOT / 'crates').rglob('*.rs')):
    if 'tests' in file.parts or file.name == 'tests.rs' or re.search(r'(?:_tests?|_test_db)\.rs$', file.name):
        continue
    source = re.sub(r'#\[cfg\(test\)\]\s*(?:#\[[^\]\n]*\]\s*)*mod \w+;', '', file.read_text())
    source = source.split('#[cfg(test)]\nmod tests {')[0]
    crate = file.relative_to(ROOT).parts[1]
    if crate in {'gateway', 'internal-api'} or str(file.relative_to(ROOT)) in {'crates/admin/src/control/http.rs', 'crates/gameauth/src/health.rs', 'crates/store/src/purchase_http.rs', 'crates/store/src/delivery_http.rs', 'crates/gameauth/src/runtime.rs'}:
        continue
    flag_source = source
    for related in [file.parent/'config.rs', file.parent/'mod.rs', file.parent.parent/(file.parent.name+'.rs')]:
        if related.is_file() and related != file:
            flag_source += related.read_text()
    flags = sorted(set(re.findall(r'"((?:OTERYN|PORTAL|PANEL)_[A-Z_]+(?:ENABLED|READ|MFA|LOGIN|PASSKEYS|TICKETS|CONSOLE|PANEL|CREDENTIALS|SESSIONS|RECOVERY_KEYS|EMAIL_CHANGE|PASSWORD_RESET|PAYMENTS|PRESENT|INGEST))"', flag_source)))
    matches = list(re.finditer(r'\.route\(\s*(?:&format!\()?"([^"\n]+)"', source))
    # Routes whose path variable comes from a literal array in the same router.
    if re.search(r'\.route\(\s*(?:path|&format!\("\{path\}/data")', source):
        for m in re.finditer(r'"(/(?:admin|console)[^"\s]*)"', source):
            if source[max(0,m.start()-250):m.start()].find('href=') < 0:
                matches.append(m)
    for m in matches:
        path = m[1]
        if not path.startswith(('/', '{prefix}')) or path.startswith('/internal/'):
            continue
        path = canonical(path)
        if crate == 'gameauth' and path == '/health':
            continue
        if path in {'/{lang}/{path}', '/{lang}/{route}'}:
            continue
        tail = ''
        if source[m.start():].startswith('.route('):
            opening = source.index('(', m.start())
            depth, quoted, escaped = 0, False, False
            for end in range(opening, len(source)):
                char = source[end]
                if quoted:
                    if escaped:
                        escaped = False
                    elif char == '\\':
                        escaped = True
                    elif char == '"':
                        quoted = False
                elif char == '"':
                    quoted = True
                elif char == '(':
                    depth += 1
                elif char == ')':
                    depth -= 1
                    if depth == 0:
                        tail = source[m.end():end]
                        break
        methods = sorted(set(x.upper() for x in re.findall(r'\b(get|post|put|delete|patch)\(', tail))) or ['GET']
        key = (crate, path)
        row = rows.setdefault(key, dict(route=path, crate=crate, flags=flags, methods=[], sources=[]))
        row['methods'] = sorted(set(row['methods'] + methods))
        row['sources'].append(f'{file.relative_to(ROOT)}:{source[:m.start()].count(chr(10))+1}')
        if '{prefix}' in m[1] and 'for prefix in ["",' in source:
            alias=canonical(m[1].replace('{prefix}', ''))
            rows.setdefault((crate,alias),dict(route=alias,crate=crate,flags=flags,methods=methods,sources=[f'{file.relative_to(ROOT)}:prefix alias']))

# Literal-array routers and a nested admin control surface.
extra = {
 'content': ['/{lang}/rules', '/{lang}/server-information', '/{lang}/legal/terms', '/{lang}/legal/privacy', '/{lang}/legal/cookies', '/{lang}/support/report-a-bug', '/{lang}/help', '/{lang}/support'],
 'gameauth': ['/api/v1/game-auth/device-sessions', '/api/v1/game-auth/device-sessions/native-characters', '/api/v1/game-auth/device-sessions/tickets', '/api/v1/game-auth/device-sessions/revoke', '/api/v1/game-auth/device-sessions/{*rest}'],
 'admin': ['/admin/moderation/reports', '/admin/moderation/reports/{id}', '/admin/moderation/enforcement', '/admin/moderation/enforcement/{id}', '/admin/analytics', '/admin/analytics/creatures', '/admin/analytics/activity', '/admin/analytics/levels', '/admin/analytics/health', '/admin/analytics/economy'],
}
for crate, paths in extra.items():
    for path in paths:
        rows.setdefault((crate,path),dict(route=path,crate=crate,flags=[],methods=['GET'],sources=['literal-array router']))
        if path.startswith('/admin/analytics'):
            rows.setdefault((crate,path+'/data'),dict(route=path+'/data',crate=crate,flags=['OTERYN_ADMIN_ANALYTICS_PRESENT'],methods=['GET'],sources=['crates/admin/src/analytics/web.rs']))
sections=[]
for file in sorted((ROOT/'crates').glob('*/src/navigation.rs')):
    source=file.read_text()
    for m in re.finditer(r'registry\.section\((.*?)\);',source,re.S):
        literals=re.findall(r'"([^"\n]+)"',m[1])
        section,label,path=literals[-3:]
        sections.append(dict(section=section,label=label,path=canonical(path),crate=file.parts[-3],group=re.search(r'Group::(\w+)',m[1])[1],footer=m[1].strip().rstrip(',').endswith('true'),source=str(file.relative_to(ROOT))))
order = re.findall(r'oteryn_(\w+)::navigation::register', (ROOT/'bin/oteryn-portal/src/main.rs').read_text())
sections.sort(key=lambda row: order.index(row['crate']) if row['crate'] in order else len(order))
for row in rows.values():
    if row['crate']=='gameauth' and 'device-sessions' in row['route'] or row['route']=='/__test/payments/webhook':
        row['methods']=['POST']
for row in rows.values():
    if re.search(r'/(admin|console)(/|$)', row['route']):
        row['flags']=sorted(set(row['flags']+['OTERYN_ADMIN_PANEL']))
    if '/admin/analytics' in row['route']:
        row['flags']=sorted(set(row['flags']+['OTERYN_ADMIN_ANALYTICS_PRESENT']))
for row in rows.values():
    if row['crate']=='bazaar' and row['route']=='/{lang}/bazaar/items/{id}/{action}':
        row['notApplicable']='Compatibility item action adapter is not mounted when the qualified Bazaar runtime is selected; current actions use account/bazaar listing bids and control endpoints (bazaar.spec.mjs). The absence is asserted as 404.'
result=dict(routes=list(rows.values()),sections=sections)
(ROOT/'e2e/inventory.json').write_text(json.dumps(result,indent=2)+'\n')
lines=['# Portal coverage inventory','','Source scan: `python3 e2e/inventory.py`. `{lang}` expands to pl and en in all four projects. Unprefixed aliases are retained separately. Source locations identify the owning router; route methods are retained in inventory.json. This inventory is provisional until the executed matrix is reconciled.','','| Route | Module crate | Enabling flags | Actor | Spec + case | Result |','| --- | --- | --- | --- | --- | --- |']
for row in result['routes']:
    actor='staff' if re.search(r'/(admin|console)(/|$)',row['route']) else 'owner' if re.search(r'/(account|panel|support/tickets|support/reports)(/|$)',row['route']) else 'anonymous/player'
    lines.append(f"| `{row['route']}` | {row['crate']} | {', '.join(row['flags']) or 'always mounted'} | {actor} | pending | NOT_RUN |")
lines+=['','## Registry sections','','| Section | Route | Group | Module | Footer | Result |','| --- | --- | --- | --- | --- | --- |']
for row in sections:
    lines.append(f"| {row['section']} | `{row['path']}` | {row['group']} | {row['crate']} | {row['footer']} | NOT_RUN |")
(ROOT/'e2e/COVERAGE.md').write_text('\n'.join(lines)+'\n')
print(f"{len(rows)} route rows; {len(sections)} registry sections")
