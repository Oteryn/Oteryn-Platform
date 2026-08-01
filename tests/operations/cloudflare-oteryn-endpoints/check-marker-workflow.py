#!/usr/bin/env python3
from pathlib import Path

workflow = Path('.github/workflows/cloudflare-oteryn-endpoints.yml').read_text(encoding='utf-8')
marker = Path('ops/triggers/cloudflare-oteryn-endpoints.md').read_text(encoding='utf-8')

required = [
    'pull_request_target:',
    'ops/triggers/cloudflare-oteryn-endpoints.md',
    "github.event.pull_request.head.repo.full_name == github.repository",
    "github.event.pull_request.base.ref == 'main'",
    'ref: ${{ github.event.pull_request.base.sha }}',
    'changed_files="$(git diff --name-only "$BASE_SHA" "$HEAD_SHA")"',
    '[[ "$changed_files" == "ops/triggers/cloudflare-oteryn-endpoints.md" ]]',
    'marker="$(git show "$HEAD_SHA:ops/triggers/cloudflare-oteryn-endpoints.md")"',
    "requested_mode='audit'",
    "requested_mode='apply'",
    "export CLOUDFLARE_APPLY_CONFIRMATION='APPLY-OTERYN-CLOUDFLARE'",
    'bash scripts/operations/cloudflare-oteryn-endpoints.sh "$requested_mode"',
    'environment: production-cloudflare',
    'permissions:\n  contents: read',
]

for invariant in required:
    if invariant not in workflow:
        raise SystemExit(f'missing trusted marker invariant: {invariant}')

if 'actions/checkout@v7' not in workflow:
    raise SystemExit('trusted marker job does not pin the checkout action major version')

if marker != '# Cloudflare Oteryn endpoint trigger\n\nmode: inert\nconfirmation:\n':
    raise SystemExit('committed endpoint marker is not inert')

if workflow.count("APPLY-OTERYN-CLOUDFLARE") < 3:
    raise SystemExit('apply confirmation is not independently enforced')

print('Trusted endpoint marker workflow boundary: PASS')
