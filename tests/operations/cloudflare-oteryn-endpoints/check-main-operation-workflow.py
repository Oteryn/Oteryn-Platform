#!/usr/bin/env python3
from pathlib import Path

workflow = Path('.github/workflows/cloudflare-oteryn-endpoint-main-operation.yml').read_text(encoding='utf-8')

required = [
    'push:',
    'branches:\n      - main',
    'ops/triggers/cloudflare-oteryn-endpoints.md',
    "if: github.event_name == 'push'",
    'environment: production-cloudflare',
    'ref: ${{ github.sha }}',
    "[[ \"$GITHUB_REF\" == 'refs/heads/main' ]]",
    "mode: inert",
    "mode: audit",
    "mode: apply",
    'confirmation: APPLY-OTERYN-CLOUDFLARE',
    "export CLOUDFLARE_APPLY_CONFIRMATION='APPLY-OTERYN-CLOUDFLARE'",
    "grep -E '^(mode|tunnel_status|tunnel_contract|www_dns|gateway_dns|legacy_gateway_dns|mutation)='",
    "ISSUE_NUMBER: '91'",
    'gh api "repos/$GITHUB_REPOSITORY/issues/$ISSUE_NUMBER/comments" -f body="$body"',
    'permissions:\n      contents: read\n      issues: write',
    'Only bounded endpoint status fields are published.',
]

for invariant in required:
    if invariant not in workflow:
        raise SystemExit(f'missing trusted-main invariant: {invariant}')

for forbidden in (
    'cat "$raw_output"',
    'body="$raw_output"',
    'body="$(cat "$raw_output")"',
    'tee endpoint-result.txt <"$raw_output"',
):
    if forbidden in workflow:
        raise SystemExit(f'raw Cloudflare output could be published: {forbidden}')

if workflow.count('APPLY-OTERYN-CLOUDFLARE') < 3:
    raise SystemExit('apply confirmation is not independently enforced')

if workflow.count('actions/checkout@v7') < 2:
    raise SystemExit('pull-request validation and trusted push do not both pin checkout')

print('Trusted-main endpoint operation workflow: PASS')
