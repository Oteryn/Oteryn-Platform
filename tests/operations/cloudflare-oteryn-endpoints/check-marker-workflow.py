#!/usr/bin/env python3
import re
from pathlib import Path

workflow = Path('.github/workflows/cloudflare-oteryn-endpoints.yml').read_text(encoding='utf-8')
marker = Path('ops/triggers/cloudflare-oteryn-endpoints.md').read_text(encoding='utf-8')

required = [
    'pull_request_target:',
    'issue_comment:',
    'ops/triggers/cloudflare-oteryn-endpoints.md',
    "github.event.pull_request.head.repo.full_name == github.repository",
    "github.event.pull_request.base.ref == 'main'",
    'ref: ${{ github.event.pull_request.base.sha }}',
    'changed_files="$(git diff --name-only "$BASE_SHA" "$HEAD_SHA")"',
    '[[ "$changed_files" == "ops/triggers/cloudflare-oteryn-endpoints.md" ]]',
    'marker="$(git show "$HEAD_SHA:ops/triggers/cloudflare-oteryn-endpoints.md")"',
    "github.actor == github.repository_owner",
    "github.event.comment.user.login == github.repository_owner",
    "/oteryn-cloudflare-endpoints audit",
    "/oteryn-cloudflare-endpoints apply APPLY-OTERYN-CLOUDFLARE",
    'pr_json="$(gh api "repos/$GITHUB_REPOSITORY/pulls/$PR_NUMBER")"',
    "[[ \"$base_ref\" == 'main' && \"$head_repo\" == \"$GITHUB_REPOSITORY\" ]]",
    "[[ \"$changed_files\" == 'ops/triggers/cloudflare-oteryn-endpoints.md' ]]",
    "requested_mode='audit'",
    "requested_mode='apply'",
    "export CLOUDFLARE_APPLY_CONFIRMATION='APPLY-OTERYN-CLOUDFLARE'",
    "grep -E '^(mode|tunnel_status|tunnel_contract|www_dns|gateway_dns|legacy_gateway_dns|mutation)='",
    'gh api "repos/$GITHUB_REPOSITORY/issues/$PR_NUMBER/comments" -f body="$body"',
    'environment: production-cloudflare',
    'permissions:\n  contents: read',
    'issues: write',
    'pull-requests: read',
]

for invariant in required:
    if invariant not in workflow:
        raise SystemExit(f'missing trusted trigger/result invariant: {invariant}')

checkout_refs = re.findall(r'^\s*uses:\s*actions/checkout@([0-9a-fA-F]{40})(?:\s+#.*)?$', workflow, re.MULTILINE)
if not checkout_refs:
    raise SystemExit('trusted operation jobs do not pin checkout to an immutable full SHA')

if marker != '# Cloudflare Oteryn endpoint trigger\n\nmode: inert\nconfirmation:\n':
    raise SystemExit('committed endpoint marker is not inert')

if workflow.count("APPLY-OTERYN-CLOUDFLARE") < 6:
    raise SystemExit('apply confirmation is not independently enforced')

for forbidden in (
    'cat "$raw_output"',
    'body="$raw_output"',
    'body="$(cat "$raw_output")"',
    'tee endpoint-result.txt <"$raw_output"',
):
    if forbidden in workflow:
        raise SystemExit(f'raw Cloudflare output could reach PR comments: {forbidden}')

if workflow.count('endpoint-result.txt') < 4:
    raise SystemExit('sanitized result file is not used by both trusted trigger paths')

print('Trusted endpoint trigger and sanitized result boundary: PASS')
