#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

legacy='login.oteryn.molehill.cloud'
unexpected=()

while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    case "$path" in
        docs/agents/reports/* | \
        docs/agents/tasks/* | \
        docs/architecture/adr/0020-use-single-level-gateway-public-hostname.md | \
        docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md | \
        deploy/synology/PUBLIC_ENDPOINTS.md | \
        docs/operations/CLOUDFLARE_EDGE_AUDIT.md | \
        docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md | \
        scripts/operations/cloudflare-oteryn-edge-audit.py | \
        scripts/operations/cloudflare-oteryn-endpoints.sh | \
        tests/operations/cloudflare-oteryn-edge-audit/* | \
        tests/operations/cloudflare-oteryn-endpoints/*)
            ;;
        *)
            unexpected+=("$path")
            ;;
    esac
done < <(git grep -l -F "$legacy" -- . || true)

if ((${#unexpected[@]} > 0)); then
    printf 'Legacy Gateway hostname appears outside migration/history allowlist:\n' >&2
    printf ' - %s\n' "${unexpected[@]}" >&2
    exit 1
fi

printf 'Legacy Gateway hostname reuse guard: PASS\n'
