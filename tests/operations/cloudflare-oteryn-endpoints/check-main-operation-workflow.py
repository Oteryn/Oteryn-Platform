#!/usr/bin/env python3
import re
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
    'Validated inert endpoint marker cleanup.',
    'An operational audit/apply marker PR may change only the marker file.',
    "[[ \"$changed_files\" == 'ops/triggers/cloudflare-oteryn-endpoints.md' ]]",
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

checkout_refs = re.findall(r'^\s*uses:\s*actions/checkout@([0-9a-fA-F]{40})(?:\s+#.*)?$', workflow, re.MULTILINE)
if len(checkout_refs) < 2 or len(set(checkout_refs)) != 1:
    raise SystemExit('pull-request validation and trusted push do not both pin the same immutable checkout SHA')

inert_index = workflow.index('if [[ "$marker" == "$inert_marker" ]]')
operational_index = workflow.index('elif [[ "$marker" == "$audit_marker" || "$marker" == "$apply_marker" ]]')
marker_only_index = workflow.index("[[ \"$changed_files\" == 'ops/triggers/cloudflare-oteryn-endpoints.md' ]]", operational_index)
if not inert_index < operational_index < marker_only_index:
    raise SystemExit('inert cleanup and marker-only operational branches are not ordered safely')

print('Trusted-main endpoint operation workflow: PASS')
