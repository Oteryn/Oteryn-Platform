#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$ROOT"

legacy='login.oteryn.molehill.cloud'
public_edge_architecture='docs/architecture/PUBLIC_EDGE_ARCHITECTURE.md'
expected_retirement='`login.oteryn.molehill.cloud` is retired and must not be restored as a canonical endpoint without an explicit contract/decision change.'
unexpected=()

# PublicEdge is the current focused architecture and must be allowed to name the
# legacy host only as an explicit retirement invariant. Fail closed if that
# statement is removed or softened instead of blindly allowlisting the file.
grep -Fxq "$expected_retirement" "$public_edge_architecture" || {
    printf 'PublicEdge architecture must preserve the exact legacy-host retirement invariant.\n' >&2
    exit 1
}

while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    case "$path" in
        docs/agents/evidence/* | \
        docs/agents/reports/* | \
        docs/agents/tasks/* | \
        docs/architecture/adr/0020-use-single-level-gateway-public-hostname.md | \
        docs/architecture/PUBLIC_EDGE_ARCHITECTURE.md | \
        docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md | \
        deploy/synology/PUBLIC_ENDPOINTS.md | \
        docs/operations/CLOUDFLARE_EDGE_AUDIT.md | \
        docs/operations/CLOUDFLARE_ENDPOINT_MANAGEMENT.md | \
        docs/operations/CLOUDFLARE_ZONE_EDGE_AUDIT.md | \
        docs/operations/OTERYN_PUBLIC_EDGE_VALIDATION.md | \
        scripts/operations/cloudflare-oteryn-edge-audit.py | \
        scripts/operations/cloudflare-oteryn-endpoints.sh | \
        scripts/operations/cloudflare-zone-edge-audit.sh | \
        tests/operations/cloudflare-oteryn-edge-audit/* | \
        tests/operations/cloudflare-oteryn-endpoints/* | \
        tests/operations/cloudflare-zone-edge-audit/*)
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
