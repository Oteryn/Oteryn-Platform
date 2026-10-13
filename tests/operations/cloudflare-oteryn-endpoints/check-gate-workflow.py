#!/usr/bin/env python3
"""Workflow-shape checks for the edge maintenance gate (textual, no YAML dependency)."""
import re
from pathlib import Path

ENDPOINTS = Path('.github/workflows/cloudflare-oteryn-endpoints.yml').read_text(encoding='utf-8')
MAIN_OP = Path('.github/workflows/cloudflare-oteryn-endpoint-main-operation.yml').read_text(encoding='utf-8')
SHARED = 'cloudflare-oteryn-edge-write'


def fail(message):
    raise SystemExit(f'gate workflow invariant failed: {message}')


def jobs(text):
    body = text.split('\njobs:\n', 1)[1]
    parts = re.split(r'(?m)^  ([a-z][a-z0-9-]*):\n', body)
    return {parts[i]: parts[i + 1] for i in range(1, len(parts), 2)}


def concurrency_group(job):
    match = re.search(r'(?m)^    concurrency:\n      group: (.+)\n      cancel-in-progress: (\w+)\n', job)
    return (match.group(1), match.group(2)) if match else None


ejobs = jobs(ENDPOINTS)
mjobs = jobs(MAIN_OP)

# Inputs and confirmation phrases.
for needle in ('- audit', '- apply', '- maintenance-close', '- maintenance-open', 'task_id:', 'closure_run_id:', 'evidence_run_id:',
               'CLOSE-OTERYN-EDGE', 'OPEN-OTERYN-EDGE', 'APPLY-OTERYN-CLOUDFLARE', r'^OTERYN-[0-9]{8}-[a-z0-9-]+$',
               'run-name: Cloudflare Oteryn Endpoints [${{ inputs.mode || github.event_name }}]'):
    if needle not in ENDPOINTS:
        fail(f'endpoints workflow is missing {needle}')

# Shared write group: exactly the write jobs, never read-only or secret-free jobs.
shared_users = set()
for name, job in {**{f'e:{k}': v for k, v in ejobs.items()}, **{f'm:{k}': v for k, v in mjobs.items()}}.items():
    group = concurrency_group(job)
    if group is None:
        if SHARED in job:
            fail(f'{name} mentions the shared group outside a concurrency block')
        continue
    value, cancel = group
    if cancel != 'false':
        fail(f'{name} must not cancel in progress')
    if SHARED in value:
        shared_users.add(name)
        if name in ('e:manage',):
            if 'format(' not in value or 'edge-audit' not in value:
                fail('manage must use a per-run group for audit')
        elif value != SHARED:
            fail(f'{name} must use the plain shared group')
    elif 'github.run_id' not in value:
        fail(f'{name} uses a non-shared group that is not per run')
expected_shared = {'e:manage', 'e:marker-apply', 'e:comment-apply', 'm:apply'}
if shared_users != expected_shared:
    fail(f'shared group users are {sorted(shared_users)}, expected {sorted(expected_shared)}')
for name in ('audit', 'edge-fingerprint', 'edge-probe', 'classify', 'guard', 'marker-audit', 'comment-audit', 'marker-guard', 'comment-guard'):
    for prefix, table in (('e', ejobs), ('m', mjobs)):
        job = table.get(name)
        if job is not None and SHARED in job:
            fail(f'{prefix}:{name} must not use the shared write group')
for prefix, table in (('e', ejobs), ('m', mjobs)):
    for name in ('audit', 'marker-audit', 'comment-audit'):
        job = table.get(name)
        if job is not None and concurrency_group(job) is None:
            fail(f'{prefix}:{name} needs a per-run group')

# Every apply-like job needs its guard; guards are read-only.
for name, guard in (('marker-apply', 'marker-guard'), ('comment-apply', 'comment-guard')):
    if guard not in ejobs[name].split('    if:', 1)[0]:
        fail(f'{name} does not need {guard}')
if 'guard' not in mjobs['apply'].split('    if:', 1)[0]:
    fail('main apply does not need guard')
if 'needs: [validate, classify, guard, edge-fingerprint]' not in ejobs['manage']:
    fail('manage does not need guard')
if 'name: manage' not in ejobs['manage'] or 'name: edge-probe' not in ejobs['edge-probe']:
    fail('manage and edge-probe job names are depended on by provenance checks')
for name, job in list(ejobs.items()) + list(mjobs.items()):
    if name.endswith('guard') or name == 'guard':
        if 'gate-guard' not in job or 'environment:' in job or 'secrets.' in job:
            fail(f'{name} must be a secret-free read-only guard')

# Gate modes only through dispatch manage.
for name, job in list(ejobs.items()) + list(mjobs.items()):
    if name.startswith(('marker-', 'comment-')) or name in mjobs:
        if 'maintenance-' in job or 'OPEN-OTERYN-EDGE' in job or 'CLOSE-OTERYN-EDGE' in job:
            if name not in ('classify',):
                fail(f'{name} must not carry gate modes')
if 'maintenance-' in MAIN_OP:
    fail('main-operation workflow must not carry gate modes')
for needle in ('/oteryn-cloudflare-endpoints maintenance', 'mode: maintenance'):
    if needle in ENDPOINTS or needle in MAIN_OP:
        fail(f'marker/comment surface carries gate mode: {needle}')

# actions: read (read-only) only on manage, edge-probe and guard jobs.
for prefix, table in (('e', ejobs), ('m', mjobs)):
    for name, job in table.items():
        has = 'actions: read' in job
        allowed = name in ('manage', 'edge-probe') or name.endswith('guard')
        if has != allowed:
            fail(f'{prefix}:{name} actions: read must be {allowed}')
if re.search(r'(?m)^permissions:\n  contents: read\n', ENDPOINTS) is None:
    fail('workflow-level permissions are not read-only')
if 'actions: ' in ENDPOINTS.split('\njobs:\n', 1)[0].split('permissions:', 1)[-1].split('concurrency:')[0].replace('actions: read', ''):
    fail('unexpected workflow-level actions permission')

# Secret-free jobs.
for name in ('edge-fingerprint', 'edge-probe', 'classify', 'validate'):
    job = ejobs[name]
    if 'secrets.' in job or 'environment:' in job or 'CLOUDFLARE_API_TOKEN' in job:
        fail(f'{name} must not use secrets or an environment')

# Artifacts.
for text, label in ((ENDPOINTS, 'endpoints'), (MAIN_OP, 'main-operation')):
    for match in re.finditer(r'uses:\s*actions/(upload|download)-artifact@(\S+)', text):
        if not re.fullmatch(r'[0-9a-f]{40}', match.group(2)):
            fail(f'{label} artifact action is not pinned to a full SHA')
for artifact in ('oteryn-edge-gate-closure', 'oteryn-edge-audit-result'):
    block = re.search(r'name: ' + artifact + r'\n(?:          .+\n)+', ENDPOINTS)
    if block is None or 'retention-days: 30' not in block.group(0):
        fail(f'{artifact} must be uploaded with retention-days: 30')
if 'upload-artifact' in MAIN_OP:
    fail('main-operation workflow uploads no gate artifacts')

# Marker/comment paths use classify then audit | guard then apply.
for prefix in ('marker', 'comment'):
    for suffix in ('classify', 'audit', 'guard', 'apply'):
        if f'{prefix}-{suffix}' not in ejobs:
            fail(f'missing job {prefix}-{suffix}')
    if 'secrets.' in ejobs[f'{prefix}-classify']:
        fail(f'{prefix}-classify must be secret-free')
for name in ('classify', 'noop', 'audit', 'guard', 'apply'):
    if name not in mjobs:
        fail(f'main-operation missing job {name}')
if 'secrets.' in mjobs['classify']:
    fail('main classify must be secret-free')

print('Maintenance gate workflow shape: PASS')
